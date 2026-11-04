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

"""Row F-4 against the default topology: a Toxiproxy timeout toxic on one
node's listener holds data with no reply and no close, as a conntrack
mismatch does. No client marks down a node the cluster still lists, so F-4
asserts bounded timeouts, no blocked tasks, recovery, and the error-rate
circuit breaker. Every toxic a case adds is removed after the row."""

import asyncio
import time
from dataclasses import dataclass

import l2_capture as capture
import l2_rows as rows
import l2_topology as topo
from aerospike_native import Client, Key, WritePolicy
from aerospike_native.exceptions import AerospikeError
from l2_load import Load, kinds, refused_by_breaker

SET = "l2f"
TOXIC = "f4-hold"
TASKS = 4
KEY_COUNT = 20
SOCKET_TIMEOUT = 500
TOTAL_TIMEOUT = 1500
MAX_RETRIES = 2
BREAKER_MAX_ERROR_RATE = 2
BREAKER_WINDOW = 4


@dataclass
class Drop:
    """The victim under load with its traffic held."""

    client: Client
    victim: topo.L2Node
    wp: WritePolicy
    keys: list[Key]
    load: Load
    injected: float
    timeout_bound: int


async def inject(policy) -> Drop:
    """Loads the victim node with writes and drops its traffic silently."""
    node_list = await topo.nodes()
    victim = node_list[-1]
    policy.address_translator = rows.port_map(topo.port_map_by_endpoint(node_list))
    client = await rows.connect(policy, [await topo.seed()])
    # Operational metrics count the breaker's rejections.
    client.enable_metrics(topo.metrics_policy())
    await topo.wait_nodes(client, await topo.node_names())
    master_list = await topo.assert_all_partitions_owned("F-4-before", client)
    keys: list[Key] = []
    i = 0
    while len(keys) < KEY_COUNT:
        k = rows.key(SET, f"f4-{i}")
        if master_list[k.partition_id].name == victim.name:
            keys.append(k)
        i += 1
    wp = rows.write_policy(socket_timeout=SOCKET_TIMEOUT, total_timeout=TOTAL_TIMEOUT, max_retries=MAX_RETRIES)

    async def write(task: int, iteration: int) -> None:
        await client.put(keys[(task + iteration * TASKS) % len(keys)], {"v": iteration}, policy=wp)

    load = Load("l2-f4", TASKS, write)
    assert await topo.wait_for(lambda: load.successes >= TASKS), "No command succeeded before the drop"
    injected = time.monotonic()
    await topo.add_toxic(victim.proxy, TOXIC, "timeout", "upstream", 1.0, {"timeout": 0})
    topo.restore_later(topo.RemoveToxic(victim.proxy, TOXIC))
    return Drop(client, victim, wp, keys, load, injected, TOTAL_TIMEOUT + SOCKET_TIMEOUT)


async def recover(drop: Drop) -> None:
    """Removes the toxic and asserts the node serves again and every
    partition is owned."""
    await drop.load.stop(drop.timeout_bound)
    await topo.remove_toxic(drop.victim.proxy, TOXIC)
    topo.restored(topo.RemoveToxic)
    removed = time.monotonic()
    client, victim = drop.client, drop.victim

    async def back() -> bool:
        listed = victim.name in topo.client_node_names(client)
        active = any(n.name == victim.name and n.is_active for n in client.nodes())
        return (
            listed
            and active
            and victim.name in topo.owners_of(client)
            and await rows.writes(client, drop.wp, drop.keys[0])
        )

    recovered = await topo.wait_for(back)
    print(
        f"L2-F4-RECOVERED back={recovered} ms={int((time.monotonic() - removed) * 1000)} "
        f"nodes={topo.client_node_names(client)} owners={topo.owners_of(client)}"
    )
    assert recovered, f"F-4: {victim.name} did not come back after the toxic was removed"
    await topo.assert_all_partitions_owned("F-4-after", client)
    await client.close()


def breaker_hits(client: Client, victim: topo.L2Node) -> int:
    """The breaker rejections the client recorded against ``victim``, from
    the operational metrics (aggregated once per tend)."""
    for node in client.nodes():
        if node.name == victim.name:
            snapshot = topo.node_snapshot(client, node)
            return 0 if snapshot is None else snapshot.circuit_breaker_hits
    return 0


async def test_f4_silent_drop_times_out_without_blocking():
    """F-4: commands to the node fail with a timeout within a bound from
    socket_timeout and total_timeout, every error is a timeout, no command
    task stays blocked past the bound, and the node serves again once the
    toxic is removed. Whether the client marks the node down is recorded,
    not asserted."""
    capture.clear()
    drop = await inject(topo.policy())
    assert await topo.wait_for(lambda: sum(1 for o in drop.load.errors() if o.timeout) >= TASKS), (
        f"F-4: fewer than {TASKS} timeouts after the drop"
    )
    finished = await drop.load.stop(drop.timeout_bound)
    errors = drop.load.errors()
    first_timeout_ms = None
    max_elapsed = 0
    for outcome in errors:
        max_elapsed = max(max_elapsed, outcome.elapsed_ms)
        assert outcome.timeout, (
            f"F-4: error is not a timeout: {outcome.kind} "
            f"{topo.messages(outcome.error) if outcome.error else ''}"
        )
        at = max(0, int((outcome.start - drop.injected) * 1000)) + outcome.elapsed_ms
        first_timeout_ms = at if first_timeout_ms is None else min(first_timeout_ms, at)
    first_timeout_ms = first_timeout_ms or 0
    refresh_failures = len(capture.messages_containing(drop.victim.name, "refresh failed"))
    print(
        f"L2-F4-TIMEOUT victim={drop.victim.name} listener={drop.victim.port} "
        f"firstTimeoutMs={first_timeout_ms} maxErrorElapsedMs={max_elapsed} "
        f"timeoutBoundMs={drop.timeout_bound} errors={kinds(errors)} "
        f"successes={drop.load.successes} tasksReturned={finished}"
    )
    print(
        f"L2-F4-NODE nodes={topo.client_node_names(drop.client)} owners={topo.owners_of(drop.client)} "
        f"refreshFailedWarnings={refresh_failures}"
    )
    assert finished, f"F-4: command tasks still blocked {drop.timeout_bound}ms after stop"
    assert first_timeout_ms <= drop.timeout_bound, (
        f"F-4: first timeout {first_timeout_ms}ms after the drop, bound {drop.timeout_bound}ms"
    )
    assert max_elapsed <= drop.timeout_bound, (
        f"F-4: an error took {max_elapsed}ms, bound {drop.timeout_bound}ms"
    )
    await recover(drop)


async def test_f4_silent_drop_trips_circuit_breaker():
    """F-4 with a low max_error_rate: the timeouts trip the node's error-rate
    circuit breaker within a bound from error_rate_window and tend_interval.
    A command started after the trip no longer waits out socket_timeout: the
    breaker refuses the node at once and the retry is served by the next
    replica, so the caller sees a fast success. A write that may not retry
    sees the rejection itself."""
    policy = topo.policy()
    policy.max_error_rate = BREAKER_MAX_ERROR_RATE
    policy.error_rate_window = BREAKER_WINDOW
    tend_interval = policy.tend_interval
    drop = await inject(policy)
    trip_bound = (BREAKER_WINDOW + 1) * tend_interval + drop.timeout_bound

    # The trip: the first breaker rejection the metrics show, or the first
    # rejection a task saw.
    def tripped() -> bool:
        return breaker_hits(drop.client, drop.victim) > 0 or any(
            o.max_error_rate for o in drop.load.errors()
        )

    tripped_at = time.monotonic() if await topo.wait_for(tripped, poll_ms=5) else None
    tripped_ms = None if tripped_at is None else int((tripped_at - drop.injected) * 1000)

    # The window resets every error_rate_window tends and the load re-trips
    # it, so a few tries may be needed to land inside a tripped window.
    no_retry = rows.write_policy(socket_timeout=SOCKET_TIMEOUT, total_timeout=TOTAL_TIMEOUT, max_retries=0)
    refused = None
    last_no_retry = None
    if tripped_at is not None:
        for _ in range(20):
            begin = time.monotonic()
            try:
                await drop.client.put(drop.keys[0], {"v": -1}, policy=no_retry)
                last_no_retry = "ok"
            except AerospikeError as e:
                if refused_by_breaker(e):
                    refused = (int((time.monotonic() - begin) * 1000), str(e))
                    break
                last_no_retry = f"{type(e).__name__}({e.result_code}) {e}"
            await asyncio.sleep(0.1)

    # Let the load run on past the trip, then measure what started after it.
    await asyncio.sleep(2 * SOCKET_TIMEOUT / 1000)
    finished = await drop.load.stop(drop.timeout_bound)
    errors = drop.load.errors()
    after_trip = [o for o in drop.load.outcomes if tripped_at is not None and o.start >= tripped_at + 0.05]
    max_after_trip_ms = max((o.elapsed_ms for o in after_trip), default=0)
    # Until the load re-trips the breaker after a window reset, a few commands
    # pay one socket_timeout again.
    fast_after_trip = sum(1 for o in after_trip if o.elapsed_ms < SOCKET_TIMEOUT)
    print(
        f"L2-F4-BREAKER victim={drop.victim.name} maxErrorRate={BREAKER_MAX_ERROR_RATE} "
        f"errorRateWindow={BREAKER_WINDOW} tendInterval={tend_interval} trippedMs={tripped_ms} "
        f"tripBoundMs={trip_bound} breakerHits={breaker_hits(drop.client, drop.victim)} "
        f"commandsAfterTrip={len(after_trip)} afterTrip={kinds(after_trip)} "
        f"fastAfterTrip={fast_after_trip} maxAfterTripElapsedMs={max_after_trip_ms} "
        f"socketTimeout={SOCKET_TIMEOUT} noRetryRefused={refused} noRetryLast={last_no_retry} "
        f"errors={kinds(errors)} "
        f"successes={drop.load.successes} tasksReturned={finished}"
    )
    assert tripped_ms is not None, f"F-4: circuit breaker never tripped: {kinds(errors)}"
    assert finished, f"F-4: command tasks still blocked {drop.timeout_bound}ms after stop"
    assert tripped_ms <= trip_bound, f"F-4: breaker tripped {tripped_ms}ms after the drop, bound {trip_bound}ms"
    assert after_trip, "F-4: no command started after the breaker tripped"
    assert fast_after_trip * 10 >= len(after_trip) * 9, (
        f"F-4: only {fast_after_trip} of {len(after_trip)} commands started after the trip finished "
        f"under socket_timeout {SOCKET_TIMEOUT}ms (max {max_after_trip_ms}ms)"
    )
    assert refused is not None, "F-4: no write with max_retries 0 was refused by the breaker"
    assert refused[0] < SOCKET_TIMEOUT, (
        f"F-4: the refused write took {refused[0]}ms, socket_timeout {SOCKET_TIMEOUT}ms: {refused[1]}"
    )
    await recover(drop)
