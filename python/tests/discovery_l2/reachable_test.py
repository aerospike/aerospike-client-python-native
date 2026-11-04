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

"""Row N-10 and the alternate list cases against the ``reachable`` profile.

The node named by DISCOVERY_REACHABLE_NODE advertises a hostname and its own
listener port as its access address; the hostname resolves to the proxy in
the client and server containers. Every other node advertises its private
address in the standard list and the same hostname with its own listener
port in the alternate list. Without the profile every case is skipped.
"""

import l2_capture as capture
import l2_rows as rows
import l2_topology as topo
from aerospike_native import ClientPolicy, Host, TranslationMissMode

BLOCKER = (
    " needs DISCOVERY_REACHABLE_NODE, a node advertising an address the client can reach "
    "(the discovery-l2 workflow's reachable profile)"
)


def require_alternates() -> None:
    rows.skip_unless(bool(topo.t().reachable_node), f"Alternate list cases{BLOCKER}")


def alternate_policy() -> ClientPolicy:
    policy = topo.policy()
    policy.use_services_alternate = True
    return policy


def reachable(node_list: list[topo.L2Node]) -> topo.L2Node:
    wanted = topo.t().reachable_node
    node = next((n for n in node_list if n.proxy == wanted), None)
    assert node is not None, f"DISCOVERY_REACHABLE_NODE {wanted} has no listener: {node_list}"
    assert node.name != node_list[0].name, f"Reachable node {node} must not be the first listener"
    return node


async def with_alternate(node_list: list[topo.L2Node]) -> list[topo.L2Node]:
    found = [n for n in node_list if await topo.alternates(n)]
    assert len(found) >= 2, f"Fewer than two nodes with an alternate address: {node_list}"
    return found


def alternate_missing(node: topo.L2Node) -> list[str]:
    """The errors that ``node`` advertises no alternate address."""
    return [e for e in capture.errors() if f"Node {node.name}" in e and "use_services_alternate" in e]


def translation_misses() -> list[str]:
    return capture.messages_containing("has no mapping")


async def assert_alternate_joins(row: str, policy: ClientPolicy) -> None:
    node_list = await topo.nodes()
    without = reachable(node_list)
    joined = await with_alternate(node_list)
    names = sorted(n.name for n in joined)
    capture.clear()

    client = await rows.connect(policy, [Host(topo.t().host, joined[0].port)])
    await topo.wait_nodes(client, names)

    def reported() -> bool:
        assert topo.client_node_names(client) == names
        return len(alternate_missing(without)) >= rows.MISS_RETRIES

    await topo.wait_for(reported)
    missing = alternate_missing(without)
    print(
        f"L2-ALT-JOIN {row} nodes={topo.client_node_names(client)} withoutAlternate={without.name} "
        f"missingErrors={len(missing)} translationMisses={len(translation_misses())}"
    )
    for expected in joined:
        host = client.get_node(expected.name).host
        alternate_list = await topo.alternates(expected)
        print(f"L2-ALT-NODE {row} {expected.name} host={host[0]}:{host[1]} alternate={alternate_list}")
        topo.assert_address(alternate_list[0], host)
    assert topo.client_node_names(client) == names
    assert len(missing) >= rows.MISS_RETRIES, (
        f"Node {without.name} without alternate not reported every tend: {capture.errors()}"
    )
    print(f"L2-ALT-LOG {row} {missing[0]}")
    assert translation_misses() == []
    await client.close()


async def assert_alternate_misses(row: str, policy: ClientPolicy, translator: str) -> None:
    joined = await with_alternate(await topo.nodes())
    seed, peer = joined[0], joined[1]
    endpoint = (await topo.alternates(peer))[0]
    capture.clear()

    client = await rows.connect(policy, [Host(topo.t().host, seed.port)])

    def missed() -> bool:
        assert topo.client_node_names(client) == [seed.name]
        return len(rows.misses(peer.name)) >= rows.MISS_RETRIES

    await topo.wait_for(missed)
    misses = rows.misses(peer.name)
    print(
        f"L2-ALT-MISS {row} peer={peer.name} nodes={topo.client_node_names(client)} "
        f"missWarnings={len(misses)}"
    )
    assert len(misses) >= rows.MISS_RETRIES, (
        f"Missed peer {peer.name} not retried every tend: {capture.warnings()}"
    )
    message = misses[0]
    print(f"L2-ALT-MISS-LOG {row} {message}")
    for part in [peer.name, endpoint, "alternate", translator, "pass_through"]:
        assert part in message, f"{part} not in: {message}"
    assert topo.client_node_names(client) == [seed.name]
    await rows.assert_seed_keys_work(row, client, seed)
    await client.close()


async def test_n10_unmapped_peer_passes_through_unchanged():
    """N-10: on_miss unset (pass_through). port-map covers every node but the
    reachable one, whose advertised address is used unchanged for its node.
    All nodes join, no translation miss is logged and the partition map is
    complete."""
    rows.skip_unless(bool(topo.t().reachable_node), f"N-10{BLOCKER}")
    node_list = await topo.nodes()
    node = reachable(node_list)
    mapped = [n for n in node_list if n.name != node.name]
    advertised = node.advertised[0]
    port_map = topo.port_map_by_endpoint(mapped)
    assert advertised not in port_map and node.name not in port_map, f"{advertised} is mapped"
    assert ClientPolicy().translation_on_miss == TranslationMissMode.PASS_THROUGH
    print(
        f"L2-N10-ADVERTISED node={node.name} container={node.proxy} "
        f"advertised={node.advertised} portMap={port_map}"
    )

    policy = topo.policy()
    policy.address_translator = rows.port_map(port_map)
    capture.clear()
    client = await rows.connect(policy, [Host(topo.t().host, mapped[0].port)])
    await topo.wait_nodes(client, await topo.node_names())
    joined = client.get_node(node.name)
    host = joined.host
    print(
        f"L2-N10-NODE {joined.name} host={host[0]}:{host[1]} active={joined.is_active} "
        f"advertised={advertised} nodes={topo.client_node_names(client)}"
    )
    assert joined.is_active
    topo.assert_address(advertised, host)
    owners = topo.owners(await topo.assert_all_partitions_owned("N-10", client))
    assert list(owners) == await topo.node_names()
    assert translation_misses() == []
    await client.close()


async def test_alternate_default_joins_nodes_with_alternate():
    """use_services_alternate with the default translator: the nodes with an
    alternate address join on it with no translation miss, and the reachable
    node, which has none, is not added and is reported on every tend."""
    require_alternates()
    await assert_alternate_joins("alternate-default", alternate_policy())


async def test_alternate_identity_ip_map_strict_joins():
    """use_services_alternate, on_miss strict and an ip_map that maps the
    alternate hostname to itself: every alternate address is mapped, so the
    nodes with one join."""
    require_alternates()
    first = (await topo.nodes())[0]
    hostname = topo.host_of((await topo.alternates(first))[0])
    policy = alternate_policy()
    policy.ip_map = {hostname: hostname}
    policy.translation_on_miss = TranslationMissMode.STRICT
    await assert_alternate_joins("alternate-ipmap-identity-strict", policy)


async def test_alternate_empty_ip_map_pass_through_joins():
    """use_services_alternate, on_miss pass_through and an empty ip_map: a
    translator that maps nothing uses every alternate address unchanged."""
    require_alternates()
    policy = alternate_policy()
    policy.ip_map = {}
    policy.translation_on_miss = TranslationMissMode.PASS_THROUGH
    await assert_alternate_joins("alternate-ipmap-empty-pass-through", policy)


async def test_alternate_empty_custom_pass_through_joins():
    """use_services_alternate, on_miss pass_through and a custom translator
    that maps nothing. The empty ip_map case covers the behavior with a
    built-in translator."""
    rows.skip_unless(False, f"alternate-custom-empty-pass-through {rows.NO_USER_TRANSLATOR}")


async def test_alternate_empty_ip_map_strict_drops_peer():
    """use_services_alternate, on_miss strict and an empty ip_map: the peer
    with an alternate address misses on the alternate list on every tend and
    only the seed is a node."""
    require_alternates()
    policy = alternate_policy()
    policy.ip_map = {}
    policy.translation_on_miss = TranslationMissMode.STRICT
    await assert_alternate_misses("alternate-ipmap-empty-strict", policy, "static-map")


async def test_alternate_empty_custom_strict_drops_peer():
    """use_services_alternate, on_miss strict and a custom translator that
    maps nothing. The empty ip_map case covers the behavior with a built-in
    translator."""
    rows.skip_unless(False, f"alternate-custom-empty-strict {rows.NO_USER_TRANSLATOR}")


async def test_alternate_seed_without_alternate_fails_init():
    """use_services_alternate seeded at the node without an alternate
    address: initialization fails with an error that names the node and the
    flag."""
    require_alternates()
    node = reachable(await topo.nodes())
    error = await rows.connect_error(
        alternate_policy(), [Host(topo.t().host, node.port)], "alternate-seed-missing"
    )
    message = str(error)
    print(f"L2-ALT-SEED-MISSING {type(error).__name__}: {message}")
    assert node.name in message, message
    assert "use_services_alternate" in message, message
