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

"""Row N-18, translation collisions, against the pass-through-ports profile.

Every node advertises one alternate-access-address hostname with its own
listener port (Branch B). The hostname resolves to the one Toxiproxy
container in the client container, so every listener shares one IP address
and the nodes differ only by port. Without DISCOVERY_PASS_THROUGH_BRANCH=B
every case is skipped.
"""

import l2_capture as capture
import l2_rows as rows
import l2_topology as topo
from aerospike_native import (
    ClientPolicy,
    Host,
    PassThroughAddressTranslator,
    Replica,
)

SET = "l2-pass-through"
KEY_COUNT = 300
COLLISION_RETRIES = 3
BRANCH_B = (
    " needs DISCOVERY_PASS_THROUGH_BRANCH=B and DISCOVERY_CONTROL_DIR, every node advertising "
    "one alternate-access-address hostname with its own port (profile pass-through-ports)"
)


def require_branch_b(row: str) -> None:
    topology = topo.t()
    rows.skip_unless(
        topology.pass_through_branch == "B" and bool(topology.control_dir), f"{row}{BRANCH_B}"
    )


def pass_through_policy() -> ClientPolicy:
    policy = topo.policy()
    policy.use_services_alternate = True
    policy.address_translator = PassThroughAddressTranslator()
    return policy


async def alternate(node: topo.L2Node) -> str:
    """The one alternate address a node advertises."""
    found = await topo.alternates(node)
    assert len(found) == 1, f"{node.name} alternate list {found}"
    return found[0]


async def seed(node: topo.L2Node) -> Host:
    endpoint = await alternate(node)
    return Host(topo.host_of(endpoint), topo.port_of(endpoint))


async def start(options: str) -> str:
    """Starts a node through the node server and registers its removal."""
    container = (await topo.control(f"start {options}"))["container"]
    topo.restore_later(topo.StopNode(container))
    return container


def collisions(node_name: str) -> list[str]:
    """The warnings that ``node_name`` collided with another node."""
    return capture.messages_containing("to the same address", node_name)


def assert_collision(message: str, node_a: str, node_b: str, host: str, translator: str) -> None:
    for part in [node_a, node_b, host, translator, "to the same address"]:
        assert part in message, f"{part} not in: {message}"


async def assert_others_serve(row: str, client, names: list[str]) -> None:
    """Every partition with a master in the client's map is owned by one of
    ``names`` and serves writes and reads, and the partitions of the
    colliding nodes have no master."""
    master_list = topo.masters(client)
    wp = rows.write_policy(replica=Replica.MASTER)
    rp = rows.read_policy(replica=Replica.MASTER)
    served = no_master = 0
    for i in range(KEY_COUNT):
        k = rows.key(SET, f"{row}-{i}")
        node = master_list[k.partition_id]
        if node is None:
            no_master += 1
            continue
        assert node.name in names, node.name
        await rows.put_get(client, wp, rp, k, i)
        served += 1
    print(f"L2-{row}-KEYS served={served} keysWithoutMaster={no_master}")
    assert served > 0
    assert no_master > 0


async def assert_colliding_nodes_create_no_node(row: str, policy: ClientPolicy, translator: str) -> None:
    node_list = await topo.nodes()
    first = node_list[0]
    names = await topo.node_names()
    hostname = topo.host_of(await alternate(first))
    port = await topo.next_listener_port()
    endpoint = f"{hostname}:{port}"
    container = await start(f"alternate-port={port}")
    await start(f"alternate-port={port}")
    await topo.wait_cluster_stable(len(node_list) + 2)
    upstream = await topo.upstream_port(first.proxy)
    await topo.add_listener_on(container, port, f"{container}:{upstream}")
    topo.restore_later(topo.RemoveListener(container))

    colliding = await topo.peer_names(first, "peers-clear-alt", endpoint)
    print(
        f"L2-{row}-ADVERTISED endpoint={endpoint} nodes={colliding} "
        f"peers-clear-alt={await topo.info_value(first, 'peers-clear-alt')}"
    )
    assert len(colliding) == 2, f"Nodes advertising {endpoint}"

    capture.clear()
    client = await rows.connect(policy, [await seed(first)])
    await topo.wait_nodes(client, names)

    def both_collided() -> bool:
        assert topo.client_node_names(client) == names
        return all(len(collisions(c)) >= COLLISION_RETRIES for c in colliding)

    await topo.wait_for(both_collided)
    message = collisions(colliding[0])[0]
    print(
        f"L2-{row}-RESULT nodes={topo.client_node_names(client)} colliding={colliding} "
        f"collisionWarnings={len(collisions(colliding[0]))}/{len(collisions(colliding[1]))}"
    )
    print(f"L2-{row}-LOG {message}")
    assert topo.client_node_names(client) == names
    assert len(collisions(colliding[1])) >= COLLISION_RETRIES, (
        f"Colliding node {colliding[1]} not retried every tend: {capture.warnings()}"
    )
    assert_collision(message, colliding[0], colliding[1], endpoint, translator)
    await assert_others_serve(row, client, names)
    await client.close()


async def assert_peers_collide_with_seed(row: str, policy: ClientPolicy, target: str, translator: str) -> None:
    """No peer becomes a node, the collision message names the peer, a node
    already at the address, the address and the translator on every tend,
    and the seed keeps serving its partitions."""
    node_list = await topo.nodes()
    first = node_list[0]
    capture.clear()
    client = await rows.connect(policy, [await seed(first)])

    def all_collided() -> bool:
        assert topo.client_node_names(client) == [first.name]
        return all(len(collisions(p.name)) >= COLLISION_RETRIES for p in node_list[1:])

    await topo.wait_for(all_collided)
    for peer in node_list[1:]:
        found = collisions(peer.name)
        print(
            f"L2-{row}-RESULT peer={peer.name} nodes={topo.client_node_names(client)} "
            f"target={target} collisionWarnings={len(found)}"
        )
        assert len(found) >= COLLISION_RETRIES, (
            f"Colliding peer {peer.name} not retried every tend: {capture.warnings()}"
        )
        message = found[0]
        print(f"L2-{row}-LOG {message}")
        # The other node named is whichever the client met first at the
        # address: the seed, or another peer translated there too.
        others = [n.name for n in node_list if n.name != peer.name]
        assert any(o in message for o in others), f"none of {others} in: {message}"
        for part in [peer.name, target, translator, "to the same address"]:
            assert part in message, f"{part} not in: {message}"
    assert topo.client_node_names(client) == [first.name]
    await rows.assert_seed_keys_work(row, client, first)
    await client.close()


async def test_n18_pass_through_colliding_nodes_create_no_node():
    """N-18 on pass-through: two nodes advertise the same
    alternate-access-address and alternate-access-port. Seeded through a node
    that does not collide, neither colliding node becomes a node, the
    collision message names both node ids, the ``host:port`` and the
    translator on every tend, and the other nodes keep serving their
    partitions."""
    require_branch_b("N-18 pass-through")
    await assert_colliding_nodes_create_no_node("N-18-pass-through", pass_through_policy(), "pass-through")


async def test_n18_static_map_colliding_nodes_create_no_node():
    """N-18 with the default translator: as on pass-through, named static-map."""
    require_branch_b("N-18 static-map")
    policy = topo.policy()
    policy.use_services_alternate = True
    await assert_colliding_nodes_create_no_node("N-18-static-map", policy, "static-map")


async def test_n18_port_map_to_seed_address_peers_create_no_node():
    """N-18 on port-map with every node's alternate address mapped to the
    seed's address: no peer becomes a node, the collision message names the
    peer, a node already at the address, the address and the translator on
    every tend, and the seed keeps serving its partitions."""
    require_branch_b("N-18 port-map to seed")
    node_list = await topo.nodes()
    first_seed = await seed(node_list[0])
    address = next(iter(topo.resolve(first_seed.name)), first_seed.name)
    target = f"{address}:{first_seed.port}"
    mapping = {await alternate(n): target for n in node_list}
    print(f"L2-N18-SEED-PORT-MAP {mapping}")
    policy = topo.policy()
    policy.use_services_alternate = True
    policy.address_translator = rows.port_map(mapping)
    await assert_peers_collide_with_seed("N-18-port-map-to-seed", policy, target, "port-map")


async def test_n18_port_map_colliding_nodes_create_no_node():
    """N-18 on port-map: two nodes are mapped to the same ``host:port``.
    Seeded through the node that does not collide, neither colliding node
    becomes a node, the collision message names both node ids, the
    ``host:port`` and the translator on every tend, and the seed keeps
    serving its partitions."""
    require_branch_b("N-18 port-map")
    node_list = await topo.nodes()
    first, a, b = node_list[0], node_list[1], node_list[2]
    mapping = topo.port_map_by_endpoint(node_list)
    for advertised in b.advertised:
        mapping[advertised] = topo.listener(a.port)
    print(f"L2-N18-PORT-MAP {mapping}")
    policy = topo.policy()
    policy.address_translator = rows.port_map(mapping)
    capture.clear()
    client = await rows.connect(policy, [await topo.seed()])

    def both_collided() -> bool:
        assert topo.client_node_names(client) == [first.name]
        return all(len(collisions(n.name)) >= COLLISION_RETRIES for n in (a, b))

    await topo.wait_for(both_collided)
    message = collisions(a.name)[0]
    print(
        f"L2-N18-PORT-MAP-RESULT nodes={topo.client_node_names(client)} colliding=[{a.name}, {b.name}] "
        f"collisionWarnings={len(collisions(a.name))}/{len(collisions(b.name))}"
    )
    print(f"L2-N18-PORT-MAP-LOG {message}")
    assert topo.client_node_names(client) == [first.name]
    assert_collision(message, a.name, b.name, topo.listener(a.port), "port-map")
    await rows.assert_seed_keys_work("N-18-port-map", client, first)
    await client.close()


async def test_n18_pass_through_newcomer_colliding_with_node_is_not_created():
    """N-18 newcomer: a node started mid-test advertises the
    alternate-access-address and port of a node that is already a node. No
    node is created for the newcomer's endpoint, under the newcomer's name
    or under the name of the node that answers on it, and the collision is
    logged on every tend. What happens to the existing node is reported, not
    asserted. The wait stops at the first extra node, since each holds
    connections to the existing node."""
    require_branch_b("N-18 newcomer")
    node_list = await topo.nodes()
    first, existing = node_list[0], node_list[-1]
    names = await topo.node_names()
    endpoint = await alternate(existing)
    client = await rows.connect(pass_through_policy(), [await seed(first)])
    await topo.wait_nodes(client, names)
    await topo.assert_all_partitions_owned("N-18-newcomer-before", client)

    capture.clear()
    await start(f"alternate-port={topo.port_of(endpoint)}")
    await topo.wait_cluster_stable(len(node_list) + 1)
    advertising = [
        n for n in await topo.peer_names(first, "peers-clear-alt", endpoint) if n != existing.name
    ]
    print(
        f"L2-N18-NEWCOMER-ADVERTISED endpoint={endpoint} existing={existing.name} "
        f"newcomer={advertising} peers-clear-alt={await topo.info_value(first, 'peers-clear-alt')}"
    )
    assert len(advertising) == 1, f"Newcomers advertising {endpoint}"
    newcomer = advertising[0]

    await topo.wait_for(
        lambda: len(collisions(newcomer)) >= COLLISION_RETRIES or len(client.nodes()) > len(names)
    )
    found = collisions(newcomer)
    node_names = topo.client_node_names(client)
    after = next((n for n in client.nodes() if n.name == existing.name), None)
    print(f"L2-N18-NEWCOMER-RESULT newcomer={newcomer} nodes={node_names} collisionWarnings={len(found)}")
    print(
        f"L2-N18-EXISTING node={existing.name} kept={existing.name in node_names} "
        f"instances={node_names.count(existing.name)} active={after is not None and after.is_active} "
        f"host={after.host if after else None} owners={topo.owners_of(client)}"
    )
    assert newcomer not in node_names, f"{newcomer} is a Node: {node_names}"
    assert node_names == names, (
        f"Node created for {newcomer}'s endpoint {endpoint}, answered by {existing.name}"
    )
    assert len(found) >= COLLISION_RETRIES, f"No collision logged for {newcomer}: {capture.warnings()}"
    print(f"L2-N18-NEWCOMER-LOG {found[0]}")
    assert_collision(found[0], newcomer, existing.name, endpoint, "pass-through")
    await client.close()
