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

"""Row N-19 against the ``port-map-range`` profile.

The nodes sit on a fixed subnet and DISCOVERY_PORT_MAP_RANGE names a
contiguous range of node addresses, including addresses no node has yet.
Toxiproxy has a listener for every address in the range on
DISCOVERY_PORT_MAP_PORT_BASE plus the address's last octet, upstream by IP.
One port-map built from that rule before the cluster scales out covers every
node started inside the range. Without the profile every case is skipped.
"""

import l2_capture as capture
import l2_rows as rows
import l2_topology as topo
from aerospike_native import ClientPolicy, TranslationMissMode

BLOCKER = (
    " needs DISCOVERY_PORT_MAP_RANGE, DISCOVERY_PORT_MAP_PORT_BASE and DISCOVERY_CONTROL_DIR, "
    "nodes on a fixed subnet with a listener per address of a range (profile port-map-range)"
)


def require_range() -> None:
    topology = topo.t()
    rows.skip_unless(
        bool(topology.port_map_range and topology.port_map_port_base and topology.control_dir),
        f"N-19{BLOCKER}",
    )


def prefix(address: str) -> str:
    return address[: address.rindex(".") + 1]


def octet(address: str) -> int:
    return int(address[address.rindex(".") + 1 :])


def address_range() -> tuple[str, str]:
    text = topo.t().port_map_range
    first, _, last = text.partition("-")
    assert prefix(first) == prefix(last), text
    return first, last


def listener_port(address: str) -> int:
    return int(topo.t().port_map_port_base) + octet(address)


def service_port(node_list: list[topo.L2Node]) -> int:
    return topo.port_of(node_list[0].advertised[0])


async def range_port_map(node_list: list[topo.L2Node]) -> dict[str, str]:
    """Builds the port-map from the rule, for every address of the range,
    and checks that the cluster's nodes are inside the range and every
    listener of the range exists."""
    first, last = address_range()
    port = service_port(node_list)
    listeners = await topo.listener_ports()
    mapping = {}
    for n in range(octet(first), octet(last) + 1):
        address = f"{prefix(first)}{n}"
        listener = listener_port(address)
        assert listener in listeners, f"No listener on {listener} for {address}: {sorted(listeners)}"
        mapping[f"{address}:{port}"] = topo.listener(listener)
    for node in node_list:
        mapped = mapping.get(node.advertised[0])
        print(f"L2-N19-NODE {node.name} advertised={node.advertised} mapped={mapped}")
        assert mapped is not None, f"{node.advertised} outside {topo.t().port_map_range}"
    print(
        f"L2-N19-PORT-MAP range={topo.t().port_map_range} portBase={topo.t().port_map_port_base} "
        f"entries={len(mapping)} {mapping}"
    )
    return mapping


def free_address(node_list: list[topo.L2Node]) -> str:
    first, last = address_range()
    port = service_port(node_list)
    used = {a for n in node_list for a in n.advertised}
    for n in range(octet(first), octet(last) + 1):
        address = f"{prefix(first)}{n}"
        if f"{address}:{port}" not in used:
            return address
    raise AssertionError(f"No free address in {topo.t().port_map_range}: {sorted(used)}")


async def start(address: str) -> dict[str, str]:
    ack = await topo.control(f"start address={address}")
    topo.restore_later(topo.StopNode(ack["container"]))
    return ack


def attempts(endpoint: str) -> list[str]:
    """The messages that the unchanged, unreachable address of a node was tried."""
    return capture.messages_containing(endpoint, "failed")


async def test_n19_prefilled_port_map_joins_node_in_range():
    """N-19: a port-map pre-filled from a fixed rule (node address to
    listener port base plus the address's last octet) for every address of
    the range joins the cluster with no warning. A node started on a free
    address of the range is an active node within 2 x tend_interval of the
    peers-generation bump that announces it, with no client configuration
    change, and the partition map is then complete with the new node among
    the owners."""
    require_range()
    node_list = await topo.nodes()
    first = node_list[0]
    names = await topo.node_names()
    mapping = await range_port_map(node_list)
    policy = topo.policy()
    policy.address_translator = rows.port_map(mapping)
    capture.clear()

    client = await rows.connect(policy, [await topo.seed()])
    await topo.wait_nodes(client, names)
    await topo.assert_all_partitions_owned("N-19-before", client)
    assert capture.warnings() == []

    address = free_address(node_list)
    generation = await topo.peers_generation(first)
    ack = await start(address)
    assert ack["address"] == address

    joined = await topo.wait_joined(client, first, generation, names)
    node = joined.node
    budget = 2 * policy.tend_interval
    port = listener_port(address)
    reply = await topo.info(topo.t().host, port, ["node", "service-clear-std"])
    mapped = mapping.get(f"{address}:{service_port(node_list)}")
    print(
        f"L2-N19-TIMING node={node.name} host={node.host} container={ack['container']} "
        f"address={address} advertised={reply['service-clear-std']} mapped={mapped} "
        f"generation={generation}->{joined.generation} bumpToActiveMs={joined.bump_to_active_ms} "
        f"tendInterval={policy.tend_interval} budgetMs={budget}"
    )
    assert reply["node"] == node.name
    assert node.host[1] == port
    assert joined.bump_to_active_ms <= budget, (
        f"{node.name} active {joined.bump_to_active_ms}ms after the bump, budget {budget}ms"
    )
    owns = await topo.wait_for(lambda: node.name in topo.owners_of(client))
    print(f"L2-N19-OWNERS {topo.owners_of(client)}")
    assert owns, f"{node.name} owns no partitions"
    after = topo.owners(await topo.assert_all_partitions_owned("N-19-after", client))
    assert len(after) == len(names) + 1
    await client.close()


async def test_n19_node_outside_range_decided_by_on_miss():
    """N-19 outside the range: a node started on an address the pre-filled
    port-map does not cover is decided by on_miss. With strict the error
    names the node, its advertised endpoint and the translator on every tend
    and no node is created. With the default the unchanged address, which
    the client cannot reach, is tried on every tend and no node is created."""
    require_range()
    node_list = await topo.nodes()
    first = node_list[0]
    names = await topo.node_names()
    mapping = await range_port_map(node_list)
    _, last = address_range()
    address = f"{prefix(last)}{octet(last) + 1}"
    advertised = f"{address}:{service_port(node_list)}"
    assert (await start(address))["address"] == address
    await topo.wait_cluster_stable(len(node_list) + 1)
    name = await topo.peer_name(first, "peers-clear-std", advertised)
    print(f"L2-N19-OUTSIDE node={name} advertised={advertised} range={topo.t().port_map_range}")
    assert name is not None, f"No peer advertises {advertised}"
    assert advertised not in mapping

    strict = topo.policy()
    strict.address_translator = rows.port_map(mapping)
    strict.translation_on_miss = TranslationMissMode.STRICT
    capture.clear()
    client = await rows.connect(strict, [await topo.seed()])

    def missed() -> bool:
        assert topo.client_node_names(client) == names
        return len(rows.misses(name)) >= rows.MISS_RETRIES

    await topo.wait_for(missed)
    misses = rows.misses(name)
    print(f"L2-N19-STRICT node={name} nodes={topo.client_node_names(client)} missWarnings={len(misses)}")
    assert len(misses) >= rows.MISS_RETRIES, f"Missed node {name} not retried every tend: {capture.warnings()}"
    assert topo.client_node_names(client) == names
    message = misses[0]
    print(f"L2-N19-STRICT-LOG {message}")
    for part in [name, advertised, "port-map", "standard"]:
        assert part in message, f"{part} not in: {message}"
    await client.close()

    pass_through = topo.policy()
    pass_through.address_translator = rows.port_map(mapping)
    assert pass_through.translation_on_miss == ClientPolicy().translation_on_miss
    assert pass_through.translation_on_miss == TranslationMissMode.PASS_THROUGH
    capture.clear()
    client = await rows.connect(pass_through, [await topo.seed()])

    def tried() -> bool:
        assert topo.client_node_names(client) == names
        return len(attempts(advertised)) >= rows.MISS_RETRIES

    await topo.wait_for(tried)
    found = attempts(advertised)
    print(
        f"L2-N19-PASS-THROUGH node={name} nodes={topo.client_node_names(client)} "
        f"attempts={len(found)} missWarnings={len(rows.misses(name))}"
    )
    assert len(found) >= rows.MISS_RETRIES, (
        f"Unchanged address {advertised} not tried every tend: {capture.warnings()}"
    )
    print(f"L2-N19-PASS-THROUGH-LOG {found[0]}")
    assert topo.client_node_names(client) == names
    assert rows.misses(name) == []
    await client.close()
