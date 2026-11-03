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

import os

import pytest
import pytest_asyncio

from aerospike_native import (
    ClientPolicy,
    Host,
    PortMapAddressTranslator,
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
