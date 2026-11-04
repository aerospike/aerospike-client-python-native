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

"""Seed discovery and address translation against a live cluster.

The seed provider tests need the server at ``AEROSPIKE_HOST``. The
translator tests need a cluster of two or more nodes, the one the
``AEROSPIKE_SC_*`` settings name, and skip otherwise: a translator only
rewrites the addresses nodes advertise for their peers. They connect over
the standard address lists, which a containerized cluster fills with
addresses the client cannot reach, so only a translation reaches the peers.
"""

import asyncio
import os
import threading
import time
from datetime import datetime, timedelta, timezone
from http.server import BaseHTTPRequestHandler, HTTPServer

import pytest
import pytest_asyncio

from aerospike_native import (
    AwsCredentials,
    AwsEc2TagSeedCandidateProvider,
    ClientPolicy,
    Host,
    PortMapAddressTranslator,
    SeedCandidateProvider,
    SeedMergePolicy,
    StaticMapAddressTranslator,
    StaticSeedCandidateProvider,
    TranslationMissMode,
    new_client,
)
from fixtures import sc_client_policy, sc_host


def _seed() -> Host:
    name, port = os.environ.get("AEROSPIKE_HOST", "localhost:3000").rsplit(":", 1)
    return Host(name, int(port))


def _policy() -> ClientPolicy:
    cp = ClientPolicy()
    cp.use_services_alternate = True
    return cp


def _periodic_policy(provider: SeedCandidateProvider) -> ClientPolicy:
    cp = _policy()
    cp.tend_interval = 250
    cp.discovery_refresh_interval = 250
    cp.seed_candidate_provider = provider
    return cp


class RecordingProvider(SeedCandidateProvider):
    """Returns the test seed, recording each refresh and each close."""

    def __init__(self, periodic=False, refresh_delay=0.0):
        self.periodic = periodic
        self.refresh_delay = refresh_delay
        self.refreshes = []
        self.closes = []

    def refresh_seed_candidates(self, ctx):
        self.refreshes.append((ctx, threading.get_native_id(), time.monotonic()))
        if len(self.refreshes) > 1:
            time.sleep(self.refresh_delay)
        return [_seed()]

    def needs_periodic_refresh(self):
        return self.periodic

    def close(self):
        self.closes.append(time.monotonic())


class TestSeedCandidateProvider:
    """A provider's seeds are the ones the client connects through."""

    async def test_discovered_seeds_replace_unreachable_static_seeds(self):
        cp = _policy()
        cp.seed_candidate_provider = StaticSeedCandidateProvider([_seed()])
        cp.discovery_merge_policy = SeedMergePolicy.DISCOVERY_ONLY
        client = await new_client(cp, [Host("127.0.0.1", 1)])
        try:
            assert client.is_connected()
        finally:
            await client.close()

    async def test_provider_supplies_every_seed(self):
        cp = _policy()
        cp.seed_candidate_provider = StaticSeedCandidateProvider([_seed()])
        client = await new_client(cp, [])
        try:
            assert client.is_connected()
        finally:
            await client.close()


class TestPythonSeedCandidateProvider:
    """A SeedCandidateProvider subclass is refreshed off the tend task and closed with the client."""

    async def test_provider_supplies_the_seeds_and_is_closed_once(self):
        provider = RecordingProvider()
        cp = _policy()
        cp.seed_candidate_provider = provider
        cp.discovery_merge_policy = SeedMergePolicy.DISCOVERY_ONLY
        client = await new_client(cp, [Host("127.0.0.1", 1)])
        try:
            assert client.is_connected()
        finally:
            await client.close()
        [(ctx, _, _)] = provider.refreshes
        assert ctx.tend_iteration == 0
        assert ctx.configured_seeds == [Host("127.0.0.1", 1)]
        assert len(provider.closes) == 1

    async def test_periodic_provider_is_refreshed_until_close(self):
        provider = RecordingProvider(periodic=True)
        client = await new_client(_periodic_policy(provider), [])
        try:
            await asyncio.sleep(1.5)
        finally:
            await client.close()
        refreshed = len(provider.refreshes)
        await asyncio.sleep(0.6)
        assert len(provider.refreshes) == refreshed
        periodic = provider.refreshes[1:]
        assert len(periodic) >= 2
        tend_iterations = [ctx.tend_iteration for ctx, _, _ in periodic]
        assert tend_iterations == sorted(tend_iterations) and tend_iterations[0] > 0
        assert len({thread for _, thread, _ in periodic}) == 1
        assert periodic[0][1] != threading.get_native_id()
        assert len(provider.closes) == 1

    async def test_close_waits_for_a_refresh_in_progress(self):
        provider = RecordingProvider(periodic=True, refresh_delay=1.0)
        client = await new_client(_periodic_policy(provider), [])
        try:
            deadline = time.monotonic() + 5
            while len(provider.refreshes) < 2:
                assert time.monotonic() < deadline, "no periodic refresh started"
                await asyncio.sleep(0.05)
        finally:
            await client.close()
        [closed_at] = provider.closes
        refresh_started = provider.refreshes[-1][2]
        assert closed_at >= refresh_started + provider.refresh_delay
        assert len(provider.refreshes) == 2


@pytest.fixture
def ec2_endpoint():
    """A local EC2 endpoint answering DescribeInstances with one instance at the test seed's address.

    Yields the endpoint URL and the requests it received, as
    ``(Authorization header, form body)`` pairs.
    """
    seed = _seed()
    body = (
        '<DescribeInstancesResponse xmlns="http://ec2.amazonaws.com/doc/2016-11-15/">'
        "<requestId>r-1</requestId><reservationSet><item><instancesSet><item>"
        f"<instanceId>i-0123</instanceId><privateIpAddress>{seed.name}</privateIpAddress>"
        "</item></instancesSet></item></reservationSet></DescribeInstancesResponse>"
    ).encode()
    requests = []

    class DescribeInstances(BaseHTTPRequestHandler):
        def do_POST(self):
            form = self.rfile.read(int(self.headers["Content-Length"])).decode()
            requests.append((self.headers.get("Authorization", ""), form))
            self.send_response(200)
            self.send_header("Content-Type", "text/xml")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):
            pass

    server = HTTPServer(("127.0.0.1", 0), DescribeInstances)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}/", requests
    finally:
        server.shutdown()
        server.server_close()


class TestAwsEc2TagSeedCandidateProvider:
    """The provider's signed DescribeInstances answer supplies the seeds."""

    async def test_instances_found_by_tag_are_the_seeds(self, ec2_endpoint):
        endpoint, requests = ec2_endpoint
        hook_calls = []

        def credentials_hook():
            hook_calls.append(time.monotonic())
            expiry = datetime.now(timezone.utc) + timedelta(hours=1)
            return AwsCredentials("AKIAEXAMPLE", "secret", "session-token", expiry)

        cp = _policy()
        cp.seed_candidate_provider = AwsEc2TagSeedCandidateProvider(
            "us-east-1",
            {"tag:aerospike-cluster": ["prod-a"]},
            port=_seed().port,
            endpoint=endpoint,
            credentials_hook=credentials_hook,
        )
        client = await new_client(cp, [])
        try:
            assert client.is_connected()
        finally:
            await client.close()
        assert len(hook_calls) == 1
        [(authorization, form)] = requests
        assert authorization.startswith("AWS4-HMAC-SHA256 Credential=AKIAEXAMPLE/")
        assert "Filter.1.Name=tag%3Aaerospike-cluster&Filter.1.Value.1=prod-a" in form
        assert "Filter.2.Name=instance-state-name&Filter.2.Value.1=running" in form


@pytest_asyncio.fixture
async def node_addresses():
    """The seed and each node's reachable address, from a two-or-more node cluster."""
    host = sc_host()
    if not host:
        pytest.skip("AEROSPIKE_SC_HOST (or AEROSPIKE_HOST) is not set")
    try:
        client = await new_client(sc_client_policy(), host)
    except Exception as exc:
        pytest.skip(f"cannot connect to the cluster at {host!r}: {exc}")
    try:
        addresses = {node.name: node.address for node in client.nodes()}
    finally:
        await client.close()
    if len(addresses) < 2:
        pytest.skip(f"translation needs two or more nodes; the cluster at {host!r} has {len(addresses)}")
    return host, addresses


class TestAddressTranslator:
    """Peers are connected at the address the translator returns."""

    async def test_port_map_by_node_name_reaches_every_node(self, node_addresses):
        host, addresses = node_addresses
        cp = sc_client_policy()
        cp.use_services_alternate = False
        cp.address_translator = PortMapAddressTranslator(addresses)
        client = await new_client(cp, host)
        try:
            assert {node.name: node.address for node in client.nodes()} == addresses
        finally:
            await client.close()

    async def test_strict_miss_leaves_the_peers_out(self, node_addresses):
        host, addresses = node_addresses
        cp = sc_client_policy()
        cp.use_services_alternate = False
        cp.address_translator = StaticMapAddressTranslator({})
        cp.translation_on_miss = TranslationMissMode.STRICT
        client = await new_client(cp, host)
        try:
            assert len(client.nodes()) == 1
        finally:
            await client.close()
