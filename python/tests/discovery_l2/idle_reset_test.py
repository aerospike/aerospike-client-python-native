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

"""Row F-1 against the default topology, in its own profile. Every toxic the
case adds is removed after the row."""

import asyncio

import l2_rows as rows
import l2_topology as topo
from aerospike_native import ClientPolicy, ReadPolicy, WritePolicy
from aerospike_native.exceptions import AerospikeError
from l2_load import Load, kinds

BLOCKER = "F-1 needs DISCOVERY_IDLE_RESET (profile idle-reset)"
SET = "l2f"
TOXIC = "f1-rst"
WARM_TASKS = 4
WARM_ITERATIONS = 50
KEY_COUNT = 100
PROBE_TIMEOUT = 5.0


async def probe(node: topo.L2Node) -> tuple[asyncio.StreamReader, asyncio.StreamWriter]:
    """Opens a connection through a listener and completes one info request
    on it, so Toxiproxy has linked it to the node."""
    reader, writer = await asyncio.wait_for(asyncio.open_connection(topo.t().host, node.port), PROBE_TIMEOUT)
    writer.write(topo.info_request(["node"]))
    await writer.drain()
    await asyncio.wait_for(topo.read_info_reply(reader), PROBE_TIMEOUT)
    return reader, writer


async def probe_result(reader: asyncio.StreamReader) -> str:
    """Reads the next byte of a probe connection: RST when the peer reset
    it, FIN when it closed it."""
    try:
        data = await asyncio.wait_for(reader.read(1), PROBE_TIMEOUT)
    except ConnectionResetError as e:
        return f"RST {e}"
    except TimeoutError:
        return "TIMEOUT"
    except OSError as e:
        return f"ERROR {e}"
    return "FIN" if not data else f"DATA {data[0]}"


async def test_f1_idle_reset_absorbed_by_pool():
    """F-1: idle flows torn down with RST, as a load balancer tears down an
    idle flow. Every listener's connections are made to close with RST
    rather than FIN, and the listeners are then cycled while the client's
    connections sit idle in the pool. With the default policy the next
    commands succeed: the pool drops the dead sockets before use, or the
    policy's retry absorbs the error, and no command fails to the caller. A
    raw probe connection on every listener and the client's connection and
    error counters prove the resets happened."""
    rows.skip_unless(bool(topo.t().idle_reset), BLOCKER)
    node_list = await topo.nodes()
    policy = topo.policy()
    policy.address_translator = rows.port_map(topo.port_map_by_endpoint(node_list))
    assert policy.idle_timeout == ClientPolicy().idle_timeout
    for node in node_list:
        await topo.add_toxic(node.proxy, TOXIC, "reset_peer", "upstream", 0.0, {"timeout": 0})
        topo.restore_later(topo.RemoveToxic(node.proxy, TOXIC))

    client = await rows.connect(policy, [await topo.seed()])
    client.enable_metrics(topo.metrics_policy())
    await topo.wait_nodes(client, await topo.node_names())
    await topo.assert_all_partitions_owned("F-1", client)
    wp, rp = WritePolicy(), ReadPolicy()

    async def warm_up(task: int, iteration: int) -> None:
        k = rows.key(SET, f"f1-{(task * WARM_ITERATIONS + iteration) % KEY_COUNT}")
        await client.put(k, {"v": iteration}, policy=wp)
        await client.get(k, policy=rp)

    warm = Load("l2-f1-warm", WARM_TASKS, warm_up)
    await topo.wait_for(lambda: len(warm.outcomes) >= WARM_TASKS * WARM_ITERATIONS)
    assert await warm.stop(topo.WAIT_MS), "Warm-up tasks blocked"
    assert not warm.errors(), f"Warm-up errors: {kinds(warm.errors())}"

    probes = [await probe(node) for node in node_list]
    idle = topo.connection_stats(client)
    pooled = sum(s.in_pool for s in idle.values())
    print(f"L2-F1-IDLE idleTimeout={policy.idle_timeout} pooled={pooled} {topo.format_stats(idle)}")
    assert pooled >= len(node_list), "No idle pooled connection to reset"

    for node in node_list:
        await topo.set_listener(node.proxy, False)
        await topo.set_listener(node.proxy, True)
    for node, (reader, writer) in zip(node_list, probes):
        result = await probe_result(reader)
        writer.close()
        print(f"L2-F1-PROBE listener={node.port} result={result}")
        assert result.startswith("RST"), (
            f"F-1: listener {node.port} did not reset an idle flow with RST: {result}"
        )

    start = topo.connection_stats(client)
    failures = []
    commands = 0
    for i in range(KEY_COUNT):
        k = rows.key(SET, f"f1-{i}")
        commands += 2
        try:
            await client.put(k, {"v": i}, policy=wp)
        except AerospikeError as e:
            failures.append(f"put {i}: {type(e).__name__} {topo.messages(e)}")
        try:
            await client.get(k, policy=rp)
        except AerospikeError as e:
            failures.append(f"get {i}: {type(e).__name__} {topo.messages(e)}")
    end = topo.connection_stats(client)
    closed = sum(max(0, s.closed - start.get(n, topo.ConnStats()).closed) for n, s in end.items())
    errors = sum(max(0, s.errors - start.get(n, topo.ConnStats()).errors) for n, s in end.items())
    print(
        f"L2-F1-RESULT commands={commands} failedToCaller={len(failures)} pooledReset={pooled} "
        f"closedDelta={closed} errorDelta={errors} writeMaxRetries={wp.max_retries} "
        f"readMaxRetries={rp.max_retries}"
    )
    print(f"L2-F1-STATS {topo.format_stats(end)}")
    for failure in failures:
        print(f"L2-F1-FAILED {failure}")
    assert closed >= pooled, f"F-1: {pooled} reset pooled connections but only {closed} closed by the client"
    assert not failures, f"F-1: commands failed to the caller after idle RST: {failures}"
    await client.close()
