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

"""Rows N-6, N-8 and N-12 against the default topology: what the client
reports and does that a driver run through the shared L2 harness does not
show, namely TLS name validation, the translation miss warning and a re-seed
that leaves the seed provider alone. Every case seeds the first listener and
builds its own policy."""

import time

import l2_capture as capture
import l2_rows as rows
import l2_topology as topo
from aerospike_native import (
    Host,
    ReadPolicy,
    SeedCandidateProvider,
    TlsConfig,
    TranslationMissMode,
    WritePolicy,
)

KEY_COUNT = 300
REFRESH_INTERVAL = 600_000


class CountingSeedProvider(SeedCandidateProvider):
    """Seed provider that returns fixed seeds and records the time of every call."""

    def __init__(self, seeds: list[Host]) -> None:
        self.calls: list[float] = []
        self.seeds = seeds

    def refresh_seed_candidates(self, ctx) -> list[Host]:
        self.calls.append(time.monotonic())
        return self.seeds

    def needs_periodic_refresh(self) -> bool:
        return True

    def calls_between(self, begin: float, end: float) -> int:
        return sum(1 for c in self.calls if begin <= c <= end)


async def assert_owned_keys_work(client, mapped_names: list[str], victim: topo.L2Node) -> None:
    """Every partition with a master is owned by one of ``mapped_names``, and
    every key is served, through the owner or a replica."""
    master_list = topo.masters(client)
    wp, rp = WritePolicy(), ReadPolicy()
    no_master = sum(1 for m in master_list if m is None)
    for node in master_list:
        if node is not None:
            assert node.name in mapped_names, node.name
    owned = missed_keys = 0
    for i in range(KEY_COUNT):
        k = rows.key(rows.SET, f"n8-{i}")
        if master_list[k.partition_id] is None:
            missed_keys += 1
        else:
            owned += 1
        await rows.put_get(client, wp, rp, k, i)
    print(
        f"L2-N8-KEYS victim={victim.name} partitionsWithoutMaster={no_master} "
        f"keysOnMappedMasters={owned} keysOnMissedPartitions={missed_keys}"
    )
    assert no_master > 0
    assert owned > 0
    assert missed_keys > 0


async def test_n6_tls_validates_against_node_tls_name():
    """N-6: TLS through the proxy, ``ee`` only. The handshake validates the
    certificate against the node's tls-name, not the endpoint hostname the
    client connects to. The server certificate carries the tls-name only as a
    dNSName SAN; the client has no CN fallback to warn about."""
    topology = topo.t()
    rows.skip_unless(
        topology.subset == "ee",
        f"N-6 needs DISCOVERY_SUBSET=ee for server TLS and TLS listeners; subset is {topology.subset}",
    )
    node_list = await topo.nodes()
    for node in node_list:
        assert node.tls_port > 0, f"No TLS listener for {node}"
        assert node.tls_advertised, f"No advertised TLS address for {node}"
        assert node.tls_name, f"No tls-name for {node}"
    first = node_list[0]
    assert first.tls_name != topology.host, f"tls-name {first.tls_name} is the endpoint hostname"
    assert topology.ca_cert, "AEROSPIKE_CA_CERT is not set; the ee topology mounts the server CA"

    policy = topo.policy()
    policy.tls_config = TlsConfig(topology.ca_cert)
    policy.address_translator = rows.port_map(topo.tls_port_map(node_list))
    capture.clear()

    client = await rows.connect(policy, [Host(topology.host, first.tls_port, first.tls_name)])
    await topo.wait_nodes(client, await topo.node_names())
    await topo.assert_all_partitions_owned("N-6", client)
    endpoint_addresses = topo.resolve(topology.host)
    for node in client.nodes():
        name, port = node.host
        expected = rows.node_of(node_list, node.name)
        print(f"L2-N6-NODE {node.name} host={name}:{port} endpoint={topology.host}{endpoint_addresses}")
        assert name == topology.host or name in endpoint_addresses, (
            f"{name} is not {topology.host}{endpoint_addresses}"
        )
        assert port == expected.tls_port
    wp, rp = WritePolicy(), ReadPolicy()
    for i in range(20):
        await rows.put_get(client, wp, rp, rows.key(rows.SET, f"n6-{i}"), i)
    await client.close()

    error = await rows.connect_error(policy, [Host(topology.host, first.tls_port, topology.host)], "N-6")
    message = topo.messages(error)
    print(f"L2-N6-ERROR {message}")
    assert "NotValidForName" in message or "certificate" in message.lower(), message
    print(f"L2-N6-CN-FALLBACK certShape={topology.tls_cert_shape} warnings={rows.cn_fallback_warnings()}")
    assert rows.cn_fallback_warnings() == []


async def test_n8_strict_miss_creates_no_node():
    """N-8: on_miss strict. A peer with no mapping produces the error naming
    the node, the advertised endpoint and the translator, and creates no
    node: its partitions have no master in the client's map. The other nodes
    keep serving every key and the missed peer is retried every tend."""
    node_list = await topo.nodes()
    victim = node_list[-1]
    mapped = [n for n in node_list if n.name != victim.name]
    mapped_names = sorted(n.name for n in mapped)
    capture.clear()

    policy = topo.policy()
    policy.address_translator = rows.port_map(topo.port_map_by_endpoint(mapped))
    policy.translation_on_miss = TranslationMissMode.STRICT

    client = await rows.connect(policy, [await topo.seed()])
    await topo.wait_nodes(client, mapped_names)

    def missed() -> bool:
        assert topo.client_node_names(client) == mapped_names
        return len(rows.misses(victim.name)) >= rows.MISS_RETRIES

    await topo.wait_for(missed)
    misses = rows.misses(victim.name)
    print(
        f"L2-N8-RESULT victim={victim.name} nodes={topo.client_node_names(client)} "
        f"missWarnings={len(misses)}"
    )
    assert len(misses) >= rows.MISS_RETRIES, (
        f"Missed peer {victim.name} not retried every tend: {capture.warnings()}"
    )
    assert topo.client_node_names(client) == mapped_names

    message = misses[0]
    print(f"L2-N8-LOG {message}")
    assert any(a in message for a in victim.advertised), f"{message} names none of {victim.advertised}"
    for part in [victim.name, "port-map", "pass_through", "standard"]:
        assert part in message, f"{part} not in: {message}"
    await assert_owned_keys_work(client, mapped_names, victim)
    await client.close()


async def test_n12_reseed_on_zero_active_nodes():
    """N-12: re-seed on zero active nodes. With every listener disabled the
    client drops to zero nodes; once they are enabled again it re-seeds from
    the seeds it holds within 2 x tend_interval plus the connect timeout,
    without a seed provider refresh, and the partition map is complete
    again."""
    node_list = await topo.nodes()
    provider = CountingSeedProvider([await topo.seed()])
    policy = topo.policy()
    policy.seed_candidate_provider = provider
    policy.discovery_refresh_interval = REFRESH_INTERVAL
    policy.address_translator = rows.port_map(topo.port_map_by_endpoint(node_list))

    client = await rows.connect(policy, [])
    names = await topo.node_names()
    await topo.wait_nodes(client, names)
    await topo.assert_all_partitions_owned("N-12-before", client)

    proxies = [p for p, _ in topo.t().proxy_ports]
    for proxy in proxies:
        topo.restore_later(topo.EnableListener(proxy))
        await topo.set_listener(proxy, False)
    assert await topo.wait_for(lambda: not client.nodes()), (
        f"Client kept nodes {topo.client_node_names(client)} with every listener disabled"
    )

    calls_before = len(provider.calls)
    reenable = time.monotonic()
    for proxy in proxies:
        await topo.set_listener(proxy, True)
    topo.restored(topo.EnableListener)

    assert await topo.wait_for(lambda: bool(client.nodes()), poll_ms=5), "Client did not re-seed"
    reseeded = time.monotonic()
    elapsed = int((reseeded - reenable) * 1000)
    budget = 2 * policy.tend_interval + policy.timeout
    refreshes = provider.calls_between(reenable, reseeded)
    print(
        f"L2-N12-RESEED reenableToFirstNodeMs={elapsed} tendInterval={policy.tend_interval} "
        f"connectTimeout={policy.timeout} budgetMs={budget} refreshesInWindow={refreshes} "
        f"refreshCallsBeforeReenable={calls_before} refreshCalls={len(provider.calls)} "
        f"refreshInterval={policy.discovery_refresh_interval} nodes={topo.client_node_names(client)}"
    )
    assert refreshes == 0
    assert len(provider.calls) == 1
    assert elapsed <= budget, f"Re-seeded {elapsed}ms after re-enable, budget {budget}ms"

    await topo.wait_nodes(client, names)
    await topo.assert_all_partitions_owned("N-12-after", client)
    await client.close()
