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


