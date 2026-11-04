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

"""Row F-3 against the ``port-range`` profile, standing in for load balancer
source NAT port exhaustion. Toxiproxy runs with a narrow local port range,
which caps the upstream connections it can open to each node. In its own
profile so that a failing F-3 does not fail the other rows."""

import asyncio
import os
import subprocess
import threading
import time
from pathlib import Path

import l2_rows as rows
import l2_topology as topo
from aerospike_native import Client, ClientPolicy, ReadPolicy, WritePolicy
from aerospike_native.exceptions import AerospikeError
from l2_load import Load, kinds

BLOCKER = (
    "F-3 needs DISCOVERY_PROXY_PORT_RANGE, Toxiproxy with a narrow net.ipv4.ip_local_port_range "
    "(profile port-range)"
)
SET = "l2ports"
KEY_COUNT = 1000
RECOVERY_KEY_COUNT = 100
SOCKET_TIMEOUT = 2000
TOTAL_TIMEOUT = 4000


def resident_mb() -> int:
    """The process's resident set in MiB: from /proc, or from ``ps`` where
    there is no /proc; 0 when neither answers."""
    try:
        for line in Path("/proc/self/status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) // 1024
    except OSError:
        pass
    try:
        out = subprocess.run(["ps", "-o", "rss=", "-p", str(os.getpid())], capture_output=True, text=True)
        return int(out.stdout.strip()) // 1024
    except (OSError, ValueError):
        return 0


def trace(client: Client, tracing: threading.Event) -> None:
    """Prints the resident set, the pool gauges and the connection counters
    every 100 ms, from a plain thread so a saturated event loop cannot
    starve the sampler."""
    started = time.monotonic()
    while not tracing.wait(0.1):
        pools = []
        for node in client.nodes():
            s = topo.node_snapshot(client, node)
            if s is None:
                continue
            pools.append(
                f"{node.name}{{inUse={s.connections_in_use},inPool={s.connections_in_pool},"
                f"recovering={s.connections_recovering},attempts={s.connections_attempts},"
                f"ok={s.connections_successful},failed={s.connections_failed},"
                f"poolEmpty={s.connections_pool_empty},closed={s.connections_closed}}}"
            )
        print(f"L2-F3-TRACE t={int((time.monotonic() - started) * 1000)}ms rss={resident_mb()}MB {' '.join(pools)}")


async def round_succeeds(client: Client, wp: WritePolicy, rp: ReadPolicy, count: int) -> bool:
    for i in range(count):
        k = rows.key(SET, f"f3-round-{i}")
        try:
            await client.put(k, {"v": i}, policy=wp)
            await client.get(k, policy=rp)
        except AerospikeError:
            return False
    return True


async def test_f3_source_port_exhaustion_surfaces_error_and_recovers():
    """F-3: with more concurrent commands than the proxy has source ports
    for, and max_conns_per_node raised above that, the client surfaces a
    clear error within a bound from socket_timeout and total_timeout rather
    than hanging, the error types and counts are observable from the client,
    and commands succeed again once the load drops."""
    rows.skip_unless(bool(topo.t().proxy_port_range), BLOCKER)
    low, high = (int(p) for p in topo.t().proxy_port_range.split())
    ports = high - low + 1
    node_list = await topo.nodes()
    tasks = 2 * ports * len(node_list)
    policy = topo.policy()
    policy.address_translator = rows.port_map(topo.port_map_by_endpoint(node_list))
    policy.max_conns_per_node = 4 * tasks
    print(
        f"L2-F3-RANGE proxyLocalPortRange={topo.t().proxy_port_range} ports={ports} "
        f"nodes={len(node_list)} tasks={tasks} maxConnsPerNode={policy.max_conns_per_node} "
        f"defaultMaxConnsPerNode={ClientPolicy().max_conns_per_node}"
    )
    client = await rows.connect(policy, [await topo.seed()])
    client.enable_metrics(topo.metrics_policy())
    await topo.wait_nodes(client, await topo.node_names())
    await topo.assert_all_partitions_owned("F-3-before", client)
    wp = rows.write_policy(socket_timeout=SOCKET_TIMEOUT, total_timeout=TOTAL_TIMEOUT)
    bound = TOTAL_TIMEOUT + SOCKET_TIMEOUT
    start = topo.connection_stats(client)
    begin = time.monotonic()

    async def write(task: int, iteration: int) -> None:
        await client.put(rows.key(SET, f"f3-{(task * 31 + iteration) % KEY_COUNT}"), {"v": iteration}, policy=wp)

    load = Load("l2-f3", tasks, write)
    stop_tracing = threading.Event()
    tracer = threading.Thread(target=trace, args=(client, stop_tracing), daemon=True)
    tracer.start()
    deadline = time.monotonic() + topo.WAIT_MS / 1000
    while load.error_count < tasks and time.monotonic() < deadline:
        await asyncio.sleep(0.05)
    print(
        f"L2-F3-WAITED t={int((time.monotonic() - begin) * 1000)}ms outcomes={load.total} "
        f"errors={load.error_count}"
    )
    finished = await load.stop(bound)
    stop_tracing.set()
    tracer.join()

    errors = load.errors()
    first_error_ms = None
    max_elapsed = 0
    timeouts = []
    for outcome in errors:
        at = max(0, int((outcome.start - begin) * 1000)) + outcome.elapsed_ms
        first_error_ms = at if first_error_ms is None else min(first_error_ms, at)
        max_elapsed = max(max_elapsed, outcome.elapsed_ms)
        if outcome.socket_timeout:
            timeouts.append(outcome)
    end = topo.connection_stats(client)
    error_delta = sum(max(0, s.errors - start.get(name, topo.ConnStats()).errors) for name, s in end.items())
    pooled = max((s.in_use + s.in_pool for s in end.values()), default=0)
    print(
        f"L2-F3-LOAD commands={len(load.outcomes)} successes={load.successes} errors={kinds(errors)} "
        f"firstErrorMs={first_error_ms} maxErrorElapsedMs={max_elapsed} boundMs={bound} "
        f"tasksReturned={finished} clientErrorDelta={error_delta} maxConnsPerNodeAfterLoad={pooled}"
    )
    print(f"L2-F3-STATS {topo.format_stats(end)}")
    if errors:
        first = errors[0]
        print(f"L2-F3-ERROR {first.kind} {topo.messages(first.error) if first.error else ''}")
    assert finished, f"F-3: command tasks still blocked {bound}ms after stop"
    assert errors, f"F-3: {tasks} tasks never exceeded the proxy's {ports} source ports per node"
    assert max_elapsed <= bound, f"F-3: an error took {max_elapsed}ms, bound {bound}ms"
    assert error_delta > 0, f"F-3: client counted no error: {topo.format_stats(end)}"
    assert not timeouts, f"F-3: source port exhaustion surfaced as a client timeout: {kinds(timeouts)}"

    rp = ReadPolicy()
    dropped = time.monotonic()

    async def recovered() -> bool:
        return all(n.is_active for n in client.nodes()) and await round_succeeds(
            client, wp, rp, 4 * len(node_list)
        )

    back = await topo.wait_for(recovered)
    recovered_ms = int((time.monotonic() - dropped) * 1000)
    failures = []
    for i in range(RECOVERY_KEY_COUNT):
        k = rows.key(SET, f"f3-{i}")
        try:
            await client.put(k, {"v": i}, policy=wp)
            record = await client.get(k, policy=rp)
            assert record.bins.get("v") == i
        except AerospikeError as e:
            failures.append(f"{i}: {type(e).__name__} {e}")
    print(
        f"L2-F3-RECOVERY recovered={back} ms={recovered_ms} keys={RECOVERY_KEY_COUNT} "
        f"failed={len(failures)} nodes={topo.client_node_names(client)}"
    )
    assert back, "F-3: no successful round of commands after the load dropped"
    assert not failures, f"F-3: commands failed after the load dropped: {failures}"
    await client.close()
