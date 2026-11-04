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

"""Row F-2 against the ``low-mtu`` profile, standing in for a private link
with a reduced MTU and no path MTU discovery. The client network is created
with a small MTU and the client writes and reads records well above it. In
its own profile so that a failing F-2 does not fail the other rows."""

import asyncio
import random
import time
from pathlib import Path

import l2_rows as rows
import l2_topology as topo
from aerospike_native import Key, exceptions
from l2_load import is_timeout

BLOCKER = "F-2 needs DISCOVERY_CLIENT_NETWORK_MTU, a client network created with a small MTU (profile low-mtu)"
SET = "l2mtu"
RECORD_SIZES = [64 * 1024, 1024 * 1024]
SOCKET_TIMEOUT = 5000
TOTAL_TIMEOUT = 10000


def key_on(master_list: list, name: str) -> Key:
    i = 0
    while True:
        k = rows.key(SET, f"f2-{i}")
        if master_list[k.partition_id].name == name:
            return k
        i += 1


async def bounded(bound_ms: int, command) -> str:
    """Runs a command with a bound and returns ``ok <ms>``, or
    ``error <ms> <message>`` for a diagnosable error. A hang past the bound
    or a client timeout fails the row."""
    begin = time.monotonic()
    try:
        await asyncio.wait_for(command, bound_ms / 1000)
    except exceptions.AerospikeError as e:
        ms = int((time.monotonic() - begin) * 1000)
        message = topo.messages(e)
        print(f"L2-F2-ERROR {ms}ms {type(e).__name__} {message}")
        assert not is_timeout(e), (
            f"F-2: client timeout instead of a diagnosable error after {ms}ms: {message}"
        )
        return f"error {ms}ms {message}"
    except TimeoutError:
        raise AssertionError(f"F-2: command still running after {bound_ms}ms") from None
    return f"ok {int((time.monotonic() - begin) * 1000)}ms"


async def test_f2_large_record_over_low_mtu_never_hangs():
    """F-2: with the client network's MTU well below the record size, a
    write and a read of a 1 MiB value through every node's listener succeed,
    or fail with a diagnosable error, within a bound from socket_timeout and
    total_timeout. A hang or a client timeout fails the row."""
    rows.skip_unless(bool(topo.t().client_network_mtu), BLOCKER)
    mtu = int(Path("/sys/class/net/eth0/mtu").read_text().strip())
    print(f"L2-F2-MTU eth0={mtu} network={topo.t().client_network_mtu}")
    assert mtu == int(topo.t().client_network_mtu)

    node_list = await topo.nodes()
    policy = topo.policy()
    policy.address_translator = rows.port_map(topo.port_map_by_endpoint(node_list))
    client = await rows.connect(policy, [await topo.seed()])
    await topo.wait_nodes(client, await topo.node_names())
    master_list = await topo.assert_all_partitions_owned("F-2", client)
    wp = rows.write_policy(socket_timeout=SOCKET_TIMEOUT, total_timeout=TOTAL_TIMEOUT)
    rp = rows.read_policy(socket_timeout=SOCKET_TIMEOUT, total_timeout=TOTAL_TIMEOUT)
    bound = TOTAL_TIMEOUT + SOCKET_TIMEOUT
    generator = random.Random(mtu)

    for node in client.nodes():
        k = key_on(master_list, node.name)
        for size in RECORD_SIZES:
            value = generator.randbytes(size)
            write = await bounded(bound, client.put(k, {"v": value}, policy=wp))

            async def read_back() -> None:
                record = await client.get(k, policy=rp)
                assert record.bins.get("v") == value, k

            read = await bounded(bound, read_back())
            print(
                f"L2-F2-RECORD node={node.name} listener={node.host[1]} mtu={mtu} bytes={size} "
                f"write={write} read={read} boundMs={bound}"
            )
    await client.close()
