# Copyright 2023-2026 Aerospike, Inc.
#
# Portions may be licensed to Aerospike, Inc. under one or more contributor
# license agreements WHICH ARE COMPATIBLE WITH THE APACHE LICENSE, VERSION 2.0.
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not
# use this file except in compliance with the License. You may obtain a copy of
# the License at http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under
# the License.

"""
Pytest configuration to automatically load environment variables from aerospike.env
"""
import logging
import os
import pytest
import pytest_asyncio
from pathlib import Path


def load_env_file(env_file_path, *, override: bool = True):
    """Load KEY=value / export KEY=value lines from a file into os.environ."""
    if not os.path.exists(env_file_path):
        return
    with open(env_file_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            if line.startswith('export '):
                line = line[7:]
            if '=' in line:
                key, value = line.split('=', 1)
                key = key.strip()
                value = value.strip().strip('"\'')
                if override or key not in os.environ:
                    os.environ[key] = value


def pytest_configure(config):
    """Called after command line options have been parsed and all plugins and initial conftest files been loaded."""
    root = Path(__file__).parent.parent
    env_local = root / "aerospike.env"
    env_example = root / "aerospike.env.example"
    if env_local.exists():
        load_env_file(env_local, override=True)
        print(f"Loaded environment variables from {env_local}\n")
    else:
        load_env_file(env_example, override=False)
        print(f"Loaded default environment variables from {env_example} (no {env_local.name})\n")

    
    # Configure logging from AEROSPIKE_LOG_LEVEL / AEROSPIKE_LOG_FILE.
    #
    # When AEROSPIKE_LOG_FILE is set we attach a dedicated file handler with a
    # timestamped format. When it is unset we only set the level on the
    # aerospike_core / aerospike_native loggers and let records propagate to
    # the root — pytest's own `log_cli` handler (see pyproject.toml) prints
    # them to stderr in the standard pytest format. Attaching a stderr
    # handler here as well would duplicate every warning on the console.
    log_level = os.environ.get("AEROSPIKE_LOG_LEVEL", "").upper()
    if log_level:
        numeric = getattr(logging, log_level, None)
        if numeric is None:
            print(f"Warning: invalid AEROSPIKE_LOG_LEVEL={log_level!r}, ignoring\n")
        else:
            log_file = os.environ.get("AEROSPIKE_LOG_FILE")
            file_handler: logging.Handler | None = None
            if log_file:
                file_handler = logging.FileHandler(log_file)
                file_handler.setFormatter(logging.Formatter(
                    "%(asctime)s %(levelname)-8s %(name)s: %(message)s",
                ))
            # ``query`` is the Rust ``log`` target for server query-plan debug
            # lines (``query_explain`` / two-phase selection), forwarded by pyo3-log.
            for prefix in ("aerospike_core", "aerospike_native", "query"):
                logger = logging.getLogger(prefix)
                logger.setLevel(numeric)
                if file_handler is not None:
                    logger.addHandler(file_handler)

    # Ensure python path includes the python directory for imports
    import sys
    python_dir = Path(__file__).parent
    if str(python_dir) not in sys.path:
        sys.path.insert(0, str(python_dir))

@pytest.fixture(scope="session")
def aerospike_host():
    """Fixture providing the Aerospike host for tests"""
    return os.environ.get('AEROSPIKE_HOST')


@pytest.fixture(scope="session")
def use_services_alternate():
    """Fixture indicating whether to use services-alternate addresses (for containerized servers)"""
    return os.environ.get('AEROSPIKE_USE_SERVICES_ALTERNATE', '').lower() == 'true'


@pytest.fixture(scope="session") 
def aerospike_host_tls():
    """Fixture providing the TLS-enabled Aerospike host for tests"""
    return os.environ.get('AEROSPIKE_HOST_TLS')


@pytest.fixture(scope="session")
def aerospike_host_sec():
    """Fixture providing the security-enabled Aerospike host for tests"""
    return os.environ.get('AEROSPIKE_HOST_SEC')


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def aerospike_host_812_required(aerospike_host, server_version):
    """The default seed, required to be server >= 8.1.2, else skip.

    Single-host model: version-gated tests connect to the default
    ``AEROSPIKE_HOST`` and skip unless it is 8.1.2+. Point ``AEROSPIKE_HOST``
    at an 8.1.2+ build to run them; CI covers the version spread via a server
    matrix rather than a dedicated host var. Skips when the seed is < 8.1.2 or
    unreachable (``server_version`` probes to ``None``).
    """
    if server_version is None or server_version < SERVER_8_1_2:
        pytest.skip(
            "default cluster is not 8.1.2+ (or unreachable); point "
            "AEROSPIKE_HOST at an 8.1.2+ build to run these tests"
        )
    return aerospike_host


# Named server-version floors for capability gates. Compare against the
# ``(M, m, p, b)`` tuple from :func:`server_version`. Centralized so feature
# checks reference an intent-named constant instead of an inline magic tuple
# (mirrors the Java clients' ``SERVER_VERSION_*`` constants). Add a new floor
# here rather than inlining a tuple in a new ``supports_*`` gate.
SERVER_8_1_2 = (8, 1, 2, 0)
SERVER_8_2_0 = (8, 2, 0, 0)


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def supports_string_operations(server_version):
    """``True`` when the (default-host) cluster supports server-side string ops.

    Covers ``StringOperation`` (strlen / substr / find / concat / upper /
    replace / pad / to_string / masking) and the string filter expressions,
    gated server-side via the Rust core's
    ``Node.version.supports_string_operations()`` (server >= 8.2.0). Single-host
    model: point ``AEROSPIKE_HOST`` at an 8.2.0+ build to exercise these; CI
    covers the version spread via a server matrix rather than a dedicated host
    var. Tests that need string ops should ``pytest.skip`` when this is
    ``False``.
    """
    return server_version is not None and server_version >= SERVER_8_2_0


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def supports_bit_b64_encode(server_version):
    """``True`` when the (default-host) cluster supports the bit b64_encode op.

    Covers ``BitOperation.b64_encode`` and the ``bit_b64_encode`` /
    ``bit_b64_encode_range`` filter expressions (server >= 8.2.0). Same
    single-host model as ``supports_string_operations``; tests should
    ``pytest.skip`` when this is ``False``.
    """
    return server_version is not None and server_version >= SERVER_8_2_0


async def _probe_all_nodes_version_capability(
    aerospike_host,
    use_services_alternate,
    capability_fn,
) -> bool:
    """``True`` when every connected node reports *capability_fn* on ``Version``."""
    from aerospike_native import ClientPolicy, new_client
    from aerospike_native.exceptions import ConnectionError

    if not aerospike_host:
        return False
    cp = ClientPolicy()
    cp.use_services_alternate = use_services_alternate
    try:
        client = await new_client(cp, aerospike_host)
    except ConnectionError:
        return False
    try:
        nodes = client.nodes()
        if not nodes:
            return False
        return all(capability_fn(n.version) for n in nodes)
    finally:
        await client.close()


def _probe_all_nodes_version_capability_blocking(
    aerospike_host,
    use_services_alternate,
    capability_fn,
) -> bool:
    """``True`` when every connected node reports *capability_fn* on ``Version``."""
    from aerospike_native import ClientPolicy, new_client_blocking
    from aerospike_native.exceptions import ConnectionError

    if not aerospike_host:
        return False
    cp = ClientPolicy()
    cp.use_services_alternate = use_services_alternate
    try:
        client = new_client_blocking(cp, aerospike_host)
    except ConnectionError:
        return False
    try:
        nodes = client.nodes()
        if not nodes:
            return False
        return all(capability_fn(n.version) for n in nodes)
    finally:
        client.close_blocking()


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def supports_server_compiled_ael(aerospike_host, use_services_alternate):
    """``True`` when every connected node supports server-compiled AEL (field 43).

    Probes ``Node.version.supports_server_compiled_ael()`` on all nodes.
    Tests that need server-compiled AEL should ``pytest.skip`` when this is
    ``False``.
    """
    return await _probe_all_nodes_version_capability(
        aerospike_host,
        use_services_alternate,
        lambda version: version.supports_server_compiled_ael(),
    )


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def supports_query_selection(aerospike_host, use_services_alternate):
    """``True`` when every connected node supports two-phase query selection.

    Probes ``Node.version.supports_query_selection()`` on all nodes (field 44
    explain → execute). Tests that need query selection should ``pytest.skip``
    when this is ``False``.
    """
    return await _probe_all_nodes_version_capability(
        aerospike_host,
        use_services_alternate,
        lambda version: version.supports_query_selection(),
    )


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def supports_vector(aerospike_host, use_services_alternate):
    """``True`` when every connected node accepts vector values and expressions.

    Probes ``Node.version.supports_vector()`` on all nodes (server >= 8.2.1).
    The client refuses vector commands to a cluster where any node lacks it.
    """
    return await _probe_all_nodes_version_capability(
        aerospike_host,
        use_services_alternate,
        lambda version: version.supports_vector(),
    )


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def supports_query_top_k(aerospike_host, use_services_alternate):
    """``True`` when every connected node supports Top-K queries.

    Probes ``Node.version.supports_query_top_k()`` on all nodes (server >= 8.2.1).
    The client refuses a Top-K query to a cluster where any node lacks it.
    """
    return await _probe_all_nodes_version_capability(
        aerospike_host,
        use_services_alternate,
        lambda version: version.supports_query_top_k(),
    )


@pytest.fixture(scope="session")
def supports_query_selection_sync(aerospike_host, use_services_alternate):
    """Sync session gate for query selection (blocking integration tests)."""
    return _probe_all_nodes_version_capability_blocking(
        aerospike_host,
        use_services_alternate,
        lambda version: version.supports_query_selection(),
    )


def _parse_build_string(build: str):
    """Parse an Aerospike server build string (e.g. ``8.1.2.1``) into a tuple.

    Returns the leading ``(major, minor, patch, build)`` quadruple. Trailing
    suffixes are tolerated to match the core's regex parser. Returns
    ``None`` if the string does not start with four dot-separated integers.
    """
    parts = build.split('.')
    if len(parts) < 4:
        return None
    try:
        head = tuple(int(p) for p in parts[:4])
    except ValueError:
        # The fourth component may carry a non-numeric suffix (e.g.
        # ``1-asdf``); slice off everything after the first run of digits.
        try:
            fourth = parts[3]
            cut = 0
            while cut < len(fourth) and fourth[cut].isdigit():
                cut += 1
            head = (int(parts[0]), int(parts[1]), int(parts[2]), int(fourth[:cut]))
        except Exception:
            return None
    return head


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def server_version(aerospike_host, use_services_alternate):
    """Probe the seed for ``build`` info and return ``(M, m, p, b)``.

    Uses the first node in the response. Returns ``None`` if the probe
    fails for any reason. Tests that need a version comparison should
    short-circuit on ``None`` (e.g. ``pytest.skip`` or fall through to
    server-side enforcement).
    """
    from aerospike_native import ClientPolicy, new_client

    if not aerospike_host:
        return None
    cp = ClientPolicy()
    cp.use_services_alternate = use_services_alternate
    try:
        client = await new_client(cp, aerospike_host)
    except Exception:
        return None
    try:
        info = await client.info("build")
    finally:
        await client.close()
    for raw in info.values():
        if not raw:
            continue
        # ``info("build")`` returns either ``{"build": "8.1.2.1"}`` (single-
        # value) or a string ``build=8.1.2.1`` depending on transport. Tolerate
        # both shapes.
        if "=" in raw:
            _, _, value = raw.partition("=")
            parsed = _parse_build_string(value.strip())
        else:
            parsed = _parse_build_string(raw.strip())
        if parsed is not None:
            return parsed
    return None


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def supports_query_ops_projection_ext(server_version):
    """``True`` when the seed cluster accepts non-basic-read ops in queries.

    Mirrors the per-node feature exposed by the Rust core's
    ``Version::supports_query_ops_projection_ext`` (server >= 8.1.2). Tests
    that need extended reads in ``Statement.set_operations`` should
    ``pytest.skip`` when this is ``False``.
    """
    return server_version is not None and server_version >= SERVER_8_1_2


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def supports_error_detail(server_version):
    """``True`` when the (default-host) cluster supplies extended error detail.

    Covers ``error_detail_verbosity`` and the resulting ``ServerError``
    ``sub_code`` / ``server_message`` / ``exp_trace``. Server >= 8.2.0; older
    servers ignore the request flags. Single-host model: point
    ``AEROSPIKE_HOST`` at an 8.2.0+ build to exercise these tests; CI covers
    the version spread via a server matrix rather than a dedicated host var.
    """
    return server_version is not None and server_version >= SERVER_8_2_0


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def enterprise(aerospike_host, use_services_alternate):
    """True when the test cluster is Enterprise Edition (queried via info).

    Tests that exercise Enterprise-only features should accept this fixture
    and ``pytest.skip`` when it's ``False``, rather than relying on a
    ``ServerError(EnterpriseOnly)`` to bubble up.
    """
    from aerospike_native import ClientPolicy, new_client

    cp = ClientPolicy()
    cp.use_services_alternate = use_services_alternate
    client = await new_client(cp, aerospike_host)
    try:
        result = await client.info("edition")
        return any("Enterprise" in v for v in result.values())
    finally:
        await client.close()
