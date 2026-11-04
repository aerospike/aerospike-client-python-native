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

import asyncio
import os
import time

import pytest
from aerospike_native import (
    AuthMode,
    ClientPolicy,
    GeoJSON,
    Key,
    PartitionFilter,
    QueryPolicy,
    ResultCode,
    Statement,
    WritePolicy,
    new_client,
)
from aerospike_native.exceptions import ServerError


_AUTH_MODES = {
    "NONE": AuthMode.NONE,
    "INTERNAL": AuthMode.INTERNAL,
    "EXTERNAL": AuthMode.EXTERNAL,
    "PKI": AuthMode.PKI,
}


def sc_client_policy() -> ClientPolicy:
    """Client policy for the cluster the ``AEROSPIKE_SC_*`` variables name."""
    cp = ClientPolicy()
    cp.use_services_alternate = (
        os.environ.get("AEROSPIKE_SC_USE_SERVICES_ALTERNATE")
        or os.environ.get("AEROSPIKE_USE_SERVICES_ALTERNATE", "")
    ).lower() == "true"
    user = os.environ.get("AEROSPIKE_SC_USER") or os.environ.get("AEROSPIKE_AUTH_USER")
    password = (
        os.environ.get("AEROSPIKE_SC_PASSWORD")
        or os.environ.get("AEROSPIKE_AUTH_PASSWORD")
    )
    mode_name = (
        os.environ.get("AEROSPIKE_SC_AUTH_MODE")
        or os.environ.get("AEROSPIKE_AUTH_MODE")
        or ""
    ).strip().upper()
    if user and password:
        mode = _AUTH_MODES.get(mode_name, AuthMode.INTERNAL)
        cp.set_auth_mode(mode, user=user, password=password)
    return cp


def sc_host() -> str:
    """Seed of the cluster the ``AEROSPIKE_SC_*`` variables name."""
    return os.environ.get("AEROSPIKE_SC_HOST") or os.environ.get("AEROSPIKE_HOST", "")


async def wait_for_index_ready(
    client,
    ns,
    set_name,
    sindex_filter,
    *,
    bins=None,
    expect_records=True,
    timeout=5.0,
    interval=0.25,
):
    """Poll until a secondary index is queryable and, by default, returns a record.

    An index that is registered and readable but not yet populated answers
    with an empty stream, so a query that does not raise is not proof the
    caller's records are visible. Pass ``expect_records=False`` when the
    filter legitimately matches nothing.
    """
    deadline = time.monotonic() + timeout
    last_err = None
    while time.monotonic() < deadline:
        try:
            stmt = Statement(ns, set_name, bins or [])
            stmt.filters = [sindex_filter]
            records = await client.query(
                stmt,
                PartitionFilter.all(),
                policy=QueryPolicy(),
            )
            async for _ in records:
                return
            if not expect_records:
                return
        except ServerError as exc:
            # Both are transient states of a just-created index: INDEX_NOT_FOUND
            # (201) = the create has not yet registered on this node, and
            # INDEX_NOT_READABLE (203) = registered but still building. Call
            # sites that own the index creation should first await the
            # IndexTask create_index returns (closing the 201 window); this
            # probe then covers the post-task readability tail — the server
            # can report the build task done before the index is queryable.
            if exc.result_code not in (
                ResultCode.INDEX_NOT_READABLE,
                ResultCode.INDEX_NOT_FOUND,
            ):
                raise
            last_err = exc
        await asyncio.sleep(interval)
    msg = f"index not readable with records within {timeout}s"
    if last_err is not None:
        raise TimeoutError(msg) from last_err
    raise TimeoutError(msg)


async def wait_for_scan_visible(
    client,
    ns,
    set_name,
    expected,
    rewrite,
    *,
    attempts=5,
    interval=0.5,
):
    """Verify a just-seeded set is scan-visible, re-issuing the writes if not.

    Truncate sets a server-side last-update-time watermark with clock-tick
    granularity; writes landing within the same tick are silently expired
    with it, so a truncate-then-seed sequence can lose every record without
    an error. Waiting alone cannot recover them — ``rewrite`` must re-issue
    the (idempotent) seed writes, which by then land past the watermark.
    """
    count = 0
    for _ in range(attempts):
        stmt = Statement(ns, set_name, [])
        records = await client.query(
            stmt,
            PartitionFilter.all(),
            policy=QueryPolicy(),
        )
        count = 0
        async for _rec in records:
            count += 1
        if count >= expected:
            return
        await asyncio.sleep(interval)
        await rewrite()
    raise TimeoutError(
        f"{ns}.{set_name}: only {count}/{expected} records scan-visible after seeding"
    )


class TestFixtureConnection:
    """Base fixture for tests that need a client connection."""

    @pytest.fixture
    async def client(self, aerospike_host, use_services_alternate):
        """Create a client connection for testing."""
        cp = ClientPolicy()
        cp.use_services_alternate = use_services_alternate
        client = await new_client(cp, aerospike_host)
        yield client
        await client.close()


class TestFixtureCleanDB(TestFixtureConnection):
    """Base fixture for tests that need a clean database."""

    @pytest.fixture
    async def client(self, aerospike_host, use_services_alternate):  # type: ignore[override]
        """Create a client connection and clean the test namespace."""
        cp = ClientPolicy()
        cp.use_services_alternate = use_services_alternate
        client = await new_client(cp, aerospike_host)
        
        # Clean the test namespace
        try:
            await client.truncate("test", "test")
        except Exception:
            # Truncate may fail due to permissions or server config, continue anyway
            pass
        # Truncate's last-update-time watermark has clock-tick granularity;
        # a write landing in the same tick is silently expired with it. Step
        # past the tick so the records this fixture's consumers write survive.
        await asyncio.sleep(0.005)
        
        yield client
        await client.close()

    @pytest.fixture
    def key(self):
        """Create a test key."""
        return Key("test", "test", 1)

    @pytest.fixture
    def key_invalid_primary_key(self):
        """Create a key with invalid primary key."""
        return Key("test", "test", 0)

    @pytest.fixture
    def key_invalid_namespace(self):
        """Create a key whose namespace does not exist on the cluster.

        Used by negative tests that verify the client surfaces a typed
        ``InvalidNamespaceError`` (and only that) when the requested
        namespace is missing from the partition map. The literal name
        ``nonexistent_ns`` is chosen so the resulting "Namespace not
        found" warnings in test output read as obviously intentional.
        """
        return Key("nonexistent_ns", "test", 1)


class TestFixtureInsertRecord(TestFixtureCleanDB):
    """Base fixture for tests that need a record inserted in the database."""

    @pytest.fixture
    def original_bin_val(self):
        """Return the original bin values that were inserted."""
        return {
            "brand": "Ford",
            "model": "Mustang",
            "year": 1964,
            "fa/ir": "بر آن مردم دیده روشنایی سلامی چو بوی خوش آشنایی",
            "mileage": 100000.1,
            "bytearray": bytearray(b'123'),
            "bytes": b'123',
            "geojson": GeoJSON('{"type":"Point","coordinates":[-80.590003, 28.60009]}')
        }

    @pytest.fixture
    # noinspection PyMethodOverriding
    async def client(self, key, original_bin_val, aerospike_host, use_services_alternate):
        """Create a client connection and insert a test record."""
        cp = ClientPolicy()
        cp.use_services_alternate = use_services_alternate
        client = await new_client(cp, aerospike_host)
        
        # Clean the test namespace - ignore errors if truncate fails
        try:
            await client.truncate("test", "test", before_nanos=0)
        except Exception:
            # Truncate may fail due to permissions or server config, continue anyway
            pass
        # Truncate's last-update-time watermark has clock-tick granularity;
        # a write landing in the same tick is silently expired with it. Step
        # past the tick so the records this fixture's consumers write survive.
        await asyncio.sleep(0.005)
        
        # Insert test record
        wp = WritePolicy()
        await client.put(key, original_bin_val, policy=wp)
        
        yield client
        await client.close()
