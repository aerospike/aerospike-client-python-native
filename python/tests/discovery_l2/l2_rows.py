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

"""Helpers the L2 rows share: keys, clients, policies and the checks that
more than one row makes."""

from typing import Optional

import l2_capture as capture
import l2_topology as topo
import pytest
from aerospike_native import (
    Client,
    ClientPolicy,
    Key,
    PortMapAddressTranslator,
    ReadPolicy,
    Replica,
    WritePolicy,
    new_client,
)
from aerospike_native.exceptions import AerospikeError

SET = "l2"
MISS_RETRIES = 3
MISS_KEY_COUNT = 200

# The rows whose translator is written in the test itself.
NO_USER_TRANSLATOR = (
    "needs a user-supplied address translator, which is out of scope for this "
    "release; the client translates with its built-in port-map, static-map and "
    "pass-through translators"
)


def skip_unless(condition: bool, reason: str) -> None:
    """Skips the row, naming what it needs, when ``condition`` does not hold."""
    if not condition:
        print(f"L2-SKIP {reason}")
        pytest.skip(reason)


def key(set_name: str, name: str) -> Key:
    return Key(topo.namespace(), set_name, name)


def write_policy(
    socket_timeout: Optional[int] = None,
    total_timeout: Optional[int] = None,
    max_retries: Optional[int] = None,
    replica: Optional[Replica] = None,
) -> WritePolicy:
    """The default write policy with the given fields changed."""
    policy = WritePolicy()
    if socket_timeout is not None:
        policy.socket_timeout = socket_timeout
    if total_timeout is not None:
        policy.total_timeout = total_timeout
    if max_retries is not None:
        policy.max_retries = max_retries
    if replica is not None:
        policy.replica = replica
    return policy


def read_policy(
    socket_timeout: Optional[int] = None,
    total_timeout: Optional[int] = None,
    replica: Optional[Replica] = None,
) -> ReadPolicy:
    """The default read policy with the given fields changed."""
    policy = ReadPolicy()
    if socket_timeout is not None:
        policy.socket_timeout = socket_timeout
    if total_timeout is not None:
        policy.total_timeout = total_timeout
    if replica is not None:
        policy.replica = replica
    return policy


async def connect(policy: ClientPolicy, seeds: list) -> Client:
    """A client the row's restore closes, failing the row when it does not connect."""
    try:
        client = await new_client(policy, seeds)
    except AerospikeError as e:
        pytest.fail(f"client did not connect: {topo.messages(e)}")
    topo.register_client(client)
    return client


async def connect_error(policy: ClientPolicy, seeds: list, row: str) -> AerospikeError:
    """The error a client construction that must fail raises."""
    try:
        client = await new_client(policy, seeds)
    except AerospikeError as e:
        return e
    names = topo.client_node_names(client)
    await client.close()
    pytest.fail(f"{row}: client constructed with nodes {names}")


def misses(victim: str) -> list[str]:
    """The warnings that a translation of ``victim`` missed."""
    return capture.messages_containing(f"has no mapping for node {victim}")


def cn_fallback_warnings() -> list[str]:
    """The warnings that a certificate matched the tls-name only by a legacy
    CN rule. The client has no such rule, so this is always empty."""
    return capture.messages_containing("legacy")


async def writes(client: Client, policy: WritePolicy, k: Key) -> bool:
    try:
        await client.put(k, {"v": 0}, policy=policy)
    except AerospikeError:
        return False
    return True


async def put_get(client: Client, wp: WritePolicy, rp: ReadPolicy, k: Key, value: int) -> None:
    """Writes ``value`` and reads it back."""
    try:
        await client.put(k, {"v": value}, policy=wp)
    except AerospikeError as e:
        pytest.fail(f"put {k}: {topo.messages(e)}")
    try:
        record = await client.get(k, policy=rp)
    except AerospikeError as e:
        pytest.fail(f"get {k}: {topo.messages(e)}")
    assert record.bins.get("v") == value, k


def node_of(node_list: list[topo.L2Node], name: str) -> topo.L2Node:
    for node in node_list:
        if node.name == name:
            return node
    raise AssertionError(f"Unknown node {name}")


def port_map(mapping: dict[str, str]) -> PortMapAddressTranslator:
    return PortMapAddressTranslator(mapping)


async def assert_port_map_joins(row: str, mapping: dict[str, str]) -> None:
    policy = topo.policy()
    print(f"L2-PORT-MAP {row} {mapping}")
    policy.address_translator = port_map(mapping)
    client = await connect(policy, [await topo.seed()])
    names = await topo.node_names()
    await topo.wait_nodes(client, names)
    print(f"L2-JOIN {row} nodes={topo.client_node_names(client)}")
    owners = topo.owners(await topo.assert_all_partitions_owned(row, client))
    assert list(owners) == names
    await client.close()


async def assert_seed_keys_work(row: str, client: Client, seed: topo.L2Node) -> None:
    """The seed serves the keys it masters, and the keys nobody masters have
    no node in the client's map."""
    master_list = topo.masters(client)
    wp, rp = WritePolicy(), ReadPolicy()
    on_seed = not_owned = 0
    for i in range(MISS_KEY_COUNT):
        k = key(SET, f"miss-{i}")
        master = master_list[k.partition_id]
        if master is None:
            not_owned += 1
            continue
        assert master.name == seed.name, k
        await put_get(client, wp, rp, k, i)
        on_seed += 1
    print(f"L2-MISS-KEYS {row} seed={seed.name} keysOnSeed={on_seed} keysNotOwned={not_owned}")
    assert on_seed > 0
    assert not_owned > 0


SERVED_STATS = [
    "client_write_success",
    "client_read_success",
    "client_proxy_complete",
    "client_proxy_error",
    "client_proxy_timeout",
]


async def assert_served_by_owners(
    row: str, client: Client, master_list: list, node_list: list[topo.L2Node], set_name: str, key_count: int
) -> None:
    """Writes and reads keys spread across partitions with replica MASTER and
    asserts, from each node's namespace statistics, that each node served
    exactly the keys it masters with no server proxy hop."""
    before = {n.name: await topo.namespace_stats(n, SERVED_STATS) for n in node_list}
    wp = write_policy(replica=Replica.MASTER)
    rp = read_policy(replica=Replica.MASTER)
    served: dict[str, int] = {}
    for i in range(key_count):
        k = key(set_name, f"{row}-{i}")
        owner = master_list[k.partition_id]
        assert owner is not None, f"{k} has no master"
        await put_get(client, wp, rp, k, i)
        served[owner.name] = served.get(owner.name, 0) + 1
    for node in node_list:
        after = await topo.namespace_stats(node, SERVED_STATS)

        def delta(name: str) -> int:
            return after[name] - before[node.name][name]

        writes_served = delta("client_write_success")
        reads_served = delta("client_read_success")
        proxied = delta("client_proxy_complete") + delta("client_proxy_error") + delta("client_proxy_timeout")
        expected = served.get(node.name, 0)
        print(
            f"L2-{row}-SERVED node={node.name} keys={expected} writes={writes_served} "
            f"reads={reads_served} proxied={proxied}"
        )
        assert expected > 0, f"{node.name} served no keys"
        assert writes_served == expected, f"{node.name} writes"
        assert reads_served == expected, f"{node.name} reads"
        assert proxied == 0, f"{node.name} proxied"
