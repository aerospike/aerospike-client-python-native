# Aerospike Python Native Client

The Python Native Client (PNC) is the ultra-high performance [PyO3](https://pyo3.rs/) layer underlying the
[Aerospike Python SDK](https://pypi.org/project/aerospike-sdk/). The PNC provides Python bindings for the Aerospike Rust client core. Both async and
blocking surfaces are made available in one client, with first-class free-threaded
Python support providing TPS and latency profiles previously unachievable in Python.

The PNC ships pre-built wheels for Linux (x86_64, aarch64), macOS (x86_64, arm64), and Windows
(x86_64) on Python 3.11–3.15, plus free-threaded 3.14 and 3.15 (`cp314t`,
`cp315t`) on Linux and macOS arm64.

> ### Important:
>
> **The PNC is not officially supported as a standalone client**;
> the APIs at this layer are undocumented.
> This package
> is the low-level primitive layer underneath the
> [Aerospike Python SDK](https://pypi.org/project/aerospike-sdk/). The
> reference sections below exist for SDK users who need to drop down to
> low-level configuration (TLS, multi-record transactions, strong-consistency
> read modes, wire compression) and for client contributors.
>
> Not to be confused with the legacy `aerospike` package, the separate C-based Python
> client. Earlier alphas were published as `aerospike-async`.
>
> Requires Aerospike Server 8.2.0+ (older servers may work but are not guaranteed).

## Resources

- **PyPI:** https://pypi.org/project/aerospike-native/
- **Source:** https://github.com/aerospike/aerospike-client-python-native
- **Issues:** https://github.com/aerospike/aerospike-client-python-native/issues
- **Releases:** https://github.com/aerospike/aerospike-client-python-native/releases

## Installation

```bash
pip install aerospike-native
```

Pin to a specific release if you need reproducible builds:

```bash
pip install aerospike-native==<version>
```

Pre-built wheels are published for every supported platform/Python combination
on regular CPython (3.11 – 3.15, ABI tags `cp311`–`cp315`) **and on the
free-threaded build** (`cp314t`, `cp315t`, Linux and macOS arm64), so no Rust toolchain
is required for ordinary use. If pip resolves to an sdist on your platform, see
[Building from source](#building-from-source) below.

## Quick start

```python
import asyncio

from aerospike_native import ClientPolicy, Key, new_client


async def main():
    client = await new_client(ClientPolicy(), "localhost:3000")

    key = Key("test", "demo", "user1")

    # Write a record (bins are plain dicts)
    await client.put(key, {"name": "Alice", "age": 28})

    # Read it back
    record = await client.get(key)
    print(record.bins)  # {'name': 'Alice', 'age': 28}

    # Read specific bins only
    record = await client.get(key, ["name"])
    print(record.bins)  # {'name': 'Alice'}

    # Delete the record
    await client.delete(key)

    await client.close()


asyncio.run(main())
```

## Blocking (sync) surface

Every async method has a `*_blocking` counterpart that blocks the calling
thread instead of returning an awaitable. Use these when you need sync
semantics without an event loop — particularly useful on free-threaded
Python, where multiple OS threads can drive blocking calls in parallel
without the GIL serializing them.

```python
from aerospike_native import ClientPolicy, Key, new_client_blocking

client = new_client_blocking(ClientPolicy(), "localhost:3000")

key = Key("test", "demo", "user1")

# Write a record
client.put_blocking(key, {"name": "Alice", "age": 28})

# Read it back
record = client.get_blocking(key)
print(record.bins)  # {'name': 'Alice', 'age': 28}

# Delete the record
client.delete_blocking(key)

client.close_blocking()
```

Naming follows `<async_method>_blocking` — `get_blocking`, `put_blocking`,
`operate_blocking`, `batch_read_blocking`, `query_blocking`, etc.

Clients created with `new_client_blocking()` are blocking-only; async
methods on them raise. If you want a single client that exposes both
sync and async ergonomics with a friendlier high-level API (sessions,
behaviors, AEL filters), use the
[Aerospike Python SDK](https://pypi.org/project/aerospike-sdk/) instead
— its `SyncClient` is built directly on top of this blocking surface.

## TLS configuration

The client supports TLS for secure connections and PKI (certificate-based)
authentication.

### Basic TLS

```python
from aerospike_native import ClientPolicy, TlsConfig, new_client

policy = ClientPolicy()
policy.tls_config = TlsConfig("path/to/ca-certificate.pem")
client = await new_client(policy, "tls-host:4333")
```

### TLS with client authentication

```python
policy = ClientPolicy()
policy.tls_config = TlsConfig.with_client_auth(
    "ca.pem",      # CA certificate
    "client.pem",  # Client certificate
    "client.key",  # Client private key
)
client = await new_client(policy, "tls-host:4333")
```

### PKI authentication

PKI mode uses client certificates for authentication (no username/password
required):

```python
from aerospike_native import AuthMode

policy = ClientPolicy()
policy.tls_config = TlsConfig.with_client_auth("ca.pem", "client.pem", "client.key")
policy.set_pki_auth()  # or: policy.set_auth_mode(AuthMode.PKI)
client = await new_client(policy, "tls-host:4333")
```

### TLS name in host strings

When the server certificate name differs from the connection hostname, specify
the TLS name:

```python
# Format: hostname:tls_name:port
# Example: connect to IP but validate certificate against "server.example.com"
client = await new_client(policy, "192.168.1.100:server.example.com:4333")
```

### Authentication modes

The client supports multiple authentication modes via `AuthMode`:

- `AuthMode.NONE` — no authentication
- `AuthMode.INTERNAL` — internal authentication (username/password)
- `AuthMode.EXTERNAL` — external authentication (LDAP, etc.)
- `AuthMode.PKI` — certificate-based authentication (requires TLS + client cert)

```python
from aerospike_native import AuthMode

policy = ClientPolicy()
policy.set_auth_mode(AuthMode.INTERNAL, user="admin", password="secret")
# or
policy.set_auth_mode(AuthMode.PKI)  # No user/password needed
```

## Seed discovery and address translation

A seed candidate provider supplies seed hosts beyond the ones passed to the
constructor, and `discovery_merge_policy` combines the two lists
(`MERGE`, the default, tries the constructor's seeds first). With a provider
set, the constructor's seed list may be empty:

```python
from aerospike_native import (
    ClientPolicy, DnsRecordType, DnsSeedCandidateProvider, SeedMergePolicy, new_client,
)

policy = ClientPolicy()
policy.seed_candidate_provider = DnsSeedCandidateProvider(
    ["_aerospike._tcp.prod.example.com"],
    record_type=DnsRecordType.SRV,
    servers=["10.20.0.2"],
)
policy.discovery_merge_policy = SeedMergePolicy.DISCOVERY_ONLY
client = await new_client(policy, [])
```

The DNS provider is resolved again every `discovery_refresh_interval` (30 s)
on the cluster tend task, and each refresh must finish within
`discovery_tend_refresh_deadline` (100 ms), so its `timeout` (80 ms) may not
exceed the deadline. `StaticSeedCandidateProvider` returns a fixed list.

For a self-managed cluster on EC2, `AwsEc2TagSeedCandidateProvider` lists
the running instances that match a set of `DescribeInstances` filters,
usually a tag, in the credentials' own account and region. It needs only the
`ec2:DescribeInstances` permission, and finds credentials the usual way
(environment, shared credentials file, EKS web identity, ECS, instance
profile). For any other source, such as SSO or an assumed role, pass a
`credentials_hook`: a callable returning `AwsCredentials`. It is called at the
first refresh and again whenever the credentials it returned are within five
minutes of their expiration, on the task running the refresh, so it should
return credentials refreshed elsewhere rather than call AWS itself:

```python
from aerospike_native import AwsCredentials, AwsEc2TagSeedCandidateProvider


def latest_role_credentials():
    c = role_session.credentials  # refreshed by the application in the background
    return AwsCredentials(c["AccessKeyId"], c["SecretAccessKey"], c["SessionToken"], c["Expiration"])


policy.seed_candidate_provider = AwsEc2TagSeedCandidateProvider(
    "us-east-1", {"tag:aerospike-cluster": ["prod-a"]}, credentials_hook=latest_role_credentials,
)
```

Like the DNS provider, it refreshes on the tend task, within its `timeout`
(80 ms); the first refresh, at construction, gets `init_timeout` (10 s).

For any other source, such as a service registry or a cloud API, subclass
`SeedCandidateProvider`. Its refreshes run on a background thread the client
starts for the provider, not on the tend task, so a refresh may block; the
tend task uses the latest successful result. A refresh that takes longer than
`discovery_refresh_timeout` (10 s) is discarded, and `client.close()` waits up
to that long for one in progress before calling the provider's `close`:

```python
from aerospike_native import Host, SeedCandidateProvider


class RegistryProvider(SeedCandidateProvider):
    def __init__(self, registry):
        self.registry = registry

    def refresh_seed_candidates(self, ctx):
        return [Host(node.ip, 3000) for node in self.registry.lookup("aerospike")]

    def needs_periodic_refresh(self):
        return True


policy.seed_candidate_provider = RegistryProvider(registry)
```

An address translator rewrites the address each node advertises for its peers
before the client connects to it, for a NAT or load balancer in front of the
cluster. `PortMapAddressTranslator` maps an advertised `host:port`, or a node
name, to a `host:port`; `StaticMapAddressTranslator` maps a host and keeps the
port, like `ip_map`; `PassThroughAddressTranslator` changes nothing. An
address with no mapping is used unchanged, or, with
`translation_on_miss = TranslationMissMode.STRICT`, not connected:

```python
from aerospike_native import PortMapAddressTranslator, TranslationMissMode

policy.address_translator = PortMapAddressTranslator({
    "10.0.0.1:3000": "lb.example.com:30001",
    "BB9020011AC4202": "lb.example.com:30002",
})
policy.translation_on_miss = TranslationMissMode.STRICT
```

A peer left out by a strict miss is logged with `ResultCode.TRANSLATION_MISS`
(-25), and one whose translated address another node already has with
`TRANSLATION_COLLISION` (-26). The constructor raises `ClientError` with
`SEED_DISCOVERY_FAILED` (-27) when the provider fails and no seed is left.

## Multi-record transactions (MRT)

Multi-record transactions require a strong-consistency namespace on the server
(Aerospike 8.0+). Group operations into a single atomic transaction by
attaching a `Txn` to each policy, then `commit` or `abort`:

```python
from aerospike_native import CommitStatus, Txn, WritePolicy
from aerospike_native.exceptions import CommitFailedError

txn = Txn()

write = WritePolicy()
write.txn = txn

try:
    await client.put(key_a, {"balance": 100}, policy=write)
    await client.put(key_b, {"balance": 200}, policy=write)
    status = await client.commit(txn)
    assert status == CommitStatus.OK
except CommitFailedError:
    await client.abort(txn)
```

MRT-specific failure result codes are exposed on `ResultCode`: `MRT_BLOCKED`,
`MRT_VERSION_MISMATCH`, `MRT_EXPIRED`, `MRT_TOO_MANY_WRITES`, `MRT_COMMITTED`,
`MRT_ABORTED`, `MRT_ALREADY_LOCKED`, `MRT_MONITOR_EXISTS`.

## Strong consistency read modes

Every read-capable policy exposes `read_mode_ap` and `read_mode_sc` for tuning
consistency on AP and SC namespaces respectively:

```python
from aerospike_native import ReadModeAP, ReadModeSC

policy = ReadPolicy()
policy.read_mode_ap = ReadModeAP.ONE          # AP namespace
policy.read_mode_sc = ReadModeSC.LINEARIZE    # SC namespace
```

## Wire-protocol compression

Every command policy (`ReadPolicy`, `WritePolicy`, `BatchPolicy`,
`QueryPolicy`) exposes a `use_compression` flag (off by default) to enable
compression of request/response payloads on the wire:

```python
policy = WritePolicy()
policy.use_compression = True
```

## Vectors and Top-K queries

Aerospike 8.2.1+ stores dense numeric vectors as bin values and ranks query
results server-side. Every cluster node must run 8.2.1 or later; otherwise the
client refuses the command before sending it.

A `Vector` holds `FLOAT16`, `INT32`, `FLOAT32` (the default), or `FLOAT64`
elements, built from a sequence or from a 1-D buffer (a numpy array,
`array.array` or `memoryview`) whose format selects the element type. It is an
immutable sequence, and `numpy.asarray(vector)` reads its elements without
copying. It is stored as a whole bin value, not nested in a list or map.

```python
from aerospike_native import (
    BinDataType, ExpOperation, ExpReadFlags, FilterExpression, Operation,
    PartitionFilter, Statement, Vector, VectorDistanceMetric,
)

await client.put(key, {"name": "lamp", "embedding": Vector([0.12, 0.98, -0.34])})
```

A Top-K query keeps the `k` best records across the whole cluster, ordered by
up to two integer, double or bytes keys. Order by a vector distance by computing it in the query's
operations and ordering on the computed bin:

```python
query = Vector([0.10, 0.95, -0.30])
metric = VectorDistanceMetric.COSINE_SIMILARITY
distance = FilterExpression.vector_distance(metric, query, FilterExpression.vector_bin("embedding"))

stmt = Statement("test", "products")
stmt.set_operations([
    Operation.get_bin("name"),
    ExpOperation.read("similarity", distance, ExpReadFlags.DEFAULT),
])
# ASC for EUCLIDEAN_SQUARED, DESC for DOT_PRODUCT and COSINE_SIMILARITY.
stmt.set_order_by("similarity", BinDataType.DOUBLE, metric.nearest_order)
stmt.top_k = 10

async for record in await client.query(stmt, PartitionFilter.all()):
    print(record.bins["name"], record.bins["similarity"])
```

Records are delivered once every node has answered. Top-K runs only through
`query` (not background queries), and is incompatible with `max_records`,
`include_bin_data=False`, and a resumed partition filter.

## Versioning

The PNC follows [SemVer](https://semver.org/). Pre-releases use the
`MAJOR.MINOR.PATCH-{alpha,beta,rc}.N` form (e.g. `1.1.0-rc.1`). PyPI
normalizes these on upload to the equivalent PEP 440 spelling (`1.1.0rc1`).

`Cargo.toml` is the single source of truth; `pyproject.toml` does **not**
duplicate the version. maturin reads it from `Cargo.toml` when it builds the
wheel, so the two are guaranteed to match.

See the [Development](#development--contributing) section for the bump
procedure.

## License

Apache License 2.0. See [LICENSE](LICENSE) for details.

---

## Development / Contributing

The sections below are for client *contributors*. Downstream users do **not**
need any of this — `pip install aerospike-native` is sufficient to use the
package.

### Prerequisites

This project uses [PyO3](https://pyo3.rs/) to build a Rust extension for
Python. You will need:

- **Python** 3.11 - 3.15, **or** 3.14t / 3.15t (free-threaded) for high-throughput / PSDK `AsyncPool` work.
  The client supports every CPython version under upstream security support; the
  floor rises in minor releases as versions reach end-of-life.
  Recommended installer: [`uv`](https://docs.astral.sh/uv/) (`uv python install 3.14.5+freethreaded`)
  or [`pyenv`](https://github.com/pyenv/pyenv) with a dedicated environment.
- **Rust toolchain** (`rustc` + `cargo`) — always required when building from source
- **Aerospike server** — required for integration tests

If Rust is not already installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify
rustc --version
cargo --version
```

### Building from source

Install Python build/test dependencies (any virtualenv works; pyenv is
recommended):

```bash
pip install -r requirements.txt
```

Build & test in one step (Rust build, stub regeneration, full test suite):

```bash
make dev-test
```

Or build only — produces a development wheel and installs it into the active
virtualenv:

```bash
make dev
```

Regenerate Python stubs (only needed after modifying Rust code):

```bash
make stubs
```

### Configuration

Copy `aerospike.env.example` to `aerospike.env` (gitignored) and edit it to
match your Aerospike database node configuration:

```bash
export AEROSPIKE_HOST=127.0.0.1:3100
```

`make test` and `make test-int` source `aerospike.env`, falling back to
`aerospike.env.example`.

### Running tests

```bash
make test          # unit + integration
make test-unit     # unit tests only (no server required)
make test-int      # integration tests only (requires running Aerospike server)
```

**macOS file descriptor limit.** On macOS, you may encounter
`ConnectionError: Failed to connect to host(s)` errors when running the full
test suite. The default file descriptor limit (256) can be exceeded by the
async client's concurrent connections.

```bash
ulimit -n 4096   # current shell session
make test
```

To make this permanent, add `ulimit -n 4096` to your shell profile (`~/.zshrc`
or `~/.bash_profile`).

### Bumping the version

Bumps are manual and happen in PRs against `stage`. Promotion workflows
(`stage → main`) do not mutate the version.

```bash
# 1. Edit Cargo.toml [package] version field, then refresh Cargo.lock:
#    e.g. 1.0.0  →  1.0.1
cargo check    # or: cargo update -p aerospike_native --precise 1.0.1

# 2. Confirm:
bin/get-version    # prints 1.0.1

# 3. Open a PR against stage with just this change.
```

### Reading the version programmatically

Anywhere a build script, CI step, or release tool needs the version:

```bash
bin/get-version    # → 1.0.0
```

The script parses the first `version` field inside the `[package]` table of
`Cargo.toml`. It has no Python or cargo runtime dependencies — usable from any
shell, container, or CI environment.
