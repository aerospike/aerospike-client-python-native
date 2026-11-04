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

"""Rows N-1 to N-14 against the default topology. N-10 is in
``reachable_test``. Every case seeds the first listener and builds its own
policy."""

import asyncio
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
DNS_BLOCKER = (
    "N-4 needs a DNS server on the L2 client network serving SRV records for the "
    "endpoint; the L2 topology has none"
)


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


async def assert_strict_miss_drops_peers(row: str, policy, translator: str) -> None:
    node_list = await topo.nodes()
    seed, peers = node_list[0], node_list[1:]
    capture.clear()

    client = await rows.connect(policy, [await topo.seed()])

    def all_missed() -> bool:
        assert topo.client_node_names(client) == [seed.name]
        return all(len(rows.misses(p.name)) >= rows.MISS_RETRIES for p in peers)

    await topo.wait_for(all_missed)
    for peer in peers:
        misses = rows.misses(peer.name)
        print(
            f"L2-MISS-RESULT {row} peer={peer.name} nodes={topo.client_node_names(client)} "
            f"missWarnings={len(misses)}"
        )
        assert len(misses) >= rows.MISS_RETRIES, (
            f"Missed peer {peer.name} not retried every tend: {capture.warnings()}"
        )
        message = misses[0]
        print(f"L2-MISS-LOG {row} {message}")
        for part in [peer.name, peer.advertised[0], "standard", translator, "pass_through"]:
            assert part in message, f"{part} not in: {message}"
    assert topo.client_node_names(client) == [seed.name]
    await rows.assert_seed_keys_work(row, client, seed)
    await client.close()


async def test_n1_baseline_without_translator_fails_on_peers():
    """N-1: baseline, no translator. The client reaches the seed through the
    endpoint and then fails on the peers, whose advertised addresses it has
    no route to."""
    topology = topo.t()
    seed = Host(topology.host, topology.ports[0])
    try:
        reply = await topo.info(seed.name, seed.port, ["node", "peers-clear-std"])
    except (OSError, AssertionError, asyncio.IncompleteReadError) as e:
        raise AssertionError(f"N-1 guard: seed {seed} is not reachable through the endpoint: {e}") from e
    node = reply.get("node", "")
    peers_reply = reply.get("peers-clear-std", "")
    peers = topo.parse_peers(peers_reply)
    print(f"L2-N1-GUARD seed={seed} node={node} peers-clear-std={peers_reply}")
    assert node, f"N-1 guard: seed {seed} returned no node name"
    assert peers, f"N-1 guard: seed {node} lists no peers: {peers_reply}"
    for peer in peers:
        for endpoint in peer.endpoints:
            await topo.assert_unreachable(endpoint)

    message = topo.messages(await rows.connect_error(topo.policy(), [seed], "N-1"))
    print(f"L2-N1-ERROR {message}")
    assert "Peers not reachable: " in message, message
    for peer in peers:
        for endpoint in peer.endpoints:
            unreachable = f"{topo.host_of(endpoint)} {topo.port_of(endpoint)}"
            assert unreachable in message, f"{unreachable} not named in: {message}"


async def test_n2_port_map_by_host_port_joins_all_nodes():
    """N-2: port-map keyed by advertised ``(host, port)``. All nodes join and
    the partition map is complete."""
    await rows.assert_port_map_joins("N-2-host-port", topo.port_map_by_endpoint(await topo.nodes()))


async def test_n2_port_map_by_node_name_joins_all_nodes():
    """N-2: port-map keyed by node name. All nodes join and the partition map
    is complete."""
    await rows.assert_port_map_joins("N-2-node-name", topo.port_map_by_node_name(await topo.nodes()))


async def test_n4_dns_provider_seeds_through_endpoint():
    """N-4: dns provider end to end. The seed comes from SRV records for the
    endpoint, one per listener, and the peers through port-map. The topology
    has no DNS server, so the row skips."""
    rows.skip_unless(False, DNS_BLOCKER)


async def test_n5_every_partition_owner_is_a_distinct_node():
    """N-5: no seed-only cluster. Every node owns partitions as its own node,
    and writes and reads for keys spread across partitions are served by the
    owner without a server proxy hop."""
    node_list = await topo.nodes()
    names = await topo.node_names()
    policy = topo.policy()
    policy.address_translator = rows.port_map(topo.port_map_by_endpoint(node_list))
    client = await rows.connect(policy, [await topo.seed()])
    await topo.wait_nodes(client, names)
    master_list = await topo.assert_all_partitions_owned("N-5", client)
    owners = topo.owners(master_list)
    print(f"L2-N5-OWNERS nodes={len(owners)} partitionsPerNode={owners}")
    assert list(owners) == names
    for node in client.nodes():
        assert client.get_node(node.name).name == node.name
    await rows.assert_served_by_owners("N5", client, master_list, node_list, rows.SET, KEY_COUNT)
    await client.close()


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


async def test_n7_node_removed_from_endpoint_degrades_and_returns():
    """N-7: node removed from the endpoint. With one node's listener disabled
    the client serves the other nodes' partitions; once it is enabled again
    the node returns and no node is stuck."""
    node_list = await topo.nodes()
    victim = node_list[-1]
    policy = topo.policy()
    policy.address_translator = rows.port_map(topo.port_map_by_endpoint(node_list))

    client = await rows.connect(policy, [await topo.seed()])
    names = await topo.node_names()
    await topo.wait_nodes(client, names)
    master_list = await topo.assert_all_partitions_owned("N-7-before", client)
    wp = rows.write_policy(max_retries=0, total_timeout=2000)
    rp = ReadPolicy()
    victim_keys, other_keys = [], []
    for i in range(KEY_COUNT):
        k = rows.key(rows.SET, f"n7-{i}")
        (victim_keys if master_list[k.partition_id].name == victim.name else other_keys).append(k)
    assert victim_keys
    assert other_keys

    topo.restore_later(topo.EnableListener(victim.proxy))
    await topo.set_listener(victim.proxy, False)

    deadline = time.monotonic() + topo.WAIT_MS / 1000
    while await rows.writes(client, wp, victim_keys[0]):
        assert time.monotonic() < deadline, f"Writes to {victim.name} still succeed with its listener disabled"
        await asyncio.sleep(0.1)

    victim_failed = 0
    for k in victim_keys:
        if not await rows.writes(client, wp, k):
            victim_failed += 1
    for k in other_keys:
        await rows.put_get(client, wp, rp, k, 1)

    active = sorted(n.name for n in client.nodes() if n.is_active and n.name != victim.name)
    others = [n for n in names if n != victim.name]
    print(
        f"L2-N7-DEGRADED victim={victim.name} nodes={topo.client_node_names(client)} "
        f"activeOthers={active} otherKeysServed={len(other_keys)} "
        f"victimKeysFailed={victim_failed}/{len(victim_keys)}"
    )
    assert active == others
    assert victim_failed == len(victim_keys)

    await topo.set_listener(victim.proxy, True)
    topo.restored(topo.EnableListener)

    deadline = time.monotonic() + topo.WAIT_MS / 1000
    while not await rows.writes(client, wp, victim_keys[0]):
        assert time.monotonic() < deadline, f"Writes to {victim.name} did not recover"
        await asyncio.sleep(0.1)
    await topo.wait_nodes(client, names)
    await topo.assert_all_partitions_owned("N-7-after", client)
    for node in client.nodes():
        assert node.is_active, f"{node.name} is not active"
    for k in victim_keys + other_keys:
        await rows.put_get(client, wp, rp, k, 2)
    print(f"L2-N7-RECOVERED nodes={topo.client_node_names(client)} keys={KEY_COUNT}")
    await client.close()


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


async def test_n11_new_node_active_within_two_tend_intervals():
    """N-11: a node started mid-test is an active node in the client within
    2 x tend_interval of the peers-generation bump that announces it. The
    row gives the node a port-map entry while the client runs, which needs a
    translator whose map the test can replace."""
    rows.skip_unless(False, f"N-11 {rows.NO_USER_TRANSLATOR}")


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


async def test_n14_use_services_alternate_without_alternate_fails_init():
    """N-14: use_services_alternate against a cluster with no alternate
    access address. The seed's service-clear-alt is empty while its standard
    peers list names the other nodes, so client initialization fails with an
    error that names the flag instead of presenting a one-node cluster."""
    first = (await topo.nodes())[0]
    alternate = await topo.info_value(first, "service-clear-alt")
    print(f"L2-N14-SEED {first.name} service-clear-alt=[{alternate}]")
    assert not alternate, alternate

    policy = topo.policy()
    policy.use_services_alternate = True
    error = await rows.connect_error(policy, [await topo.seed()], "N-14")
    message = str(error)
    print(f"L2-N14-ERROR {type(error).__name__}: {message}")
    assert "use_services_alternate" in message, message
    assert first.name in message, message


async def test_strict_empty_ip_map_drops_peers():
    """on_miss strict with an empty static-map (ip_map): every peer misses,
    the error names the peer, its advertised endpoint, the standard list and
    the translator, and is repeated every tend. Only the seed is a node and
    serves the keys it holds."""
    policy = topo.policy()
    policy.ip_map = {}
    policy.translation_on_miss = TranslationMissMode.STRICT
    await assert_strict_miss_drops_peers("ipmap-empty-strict", policy, "static-map")


async def test_strict_empty_custom_translator_drops_peers():
    """on_miss strict with a translator that maps nothing, as with the empty
    ip_map but named by the translator. The empty ip_map case covers the
    behavior with a built-in translator."""
    rows.skip_unless(False, f"custom-empty-strict {rows.NO_USER_TRANSLATOR}")

