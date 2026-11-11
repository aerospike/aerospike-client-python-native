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

import asyncio
import time

import pytest
from aerospike_native import (
    ClientPolicy,
    new_client,
    new_client_blocking,
    Statement,
    Recordset,
    Record,
    QueryPolicy,
    ReadPolicy,
    PartitionFilter,
    Filter,
    IndexType,
    CollectionIndexType,
    FilterExpression,
    CTX,
    Key,
    WritePolicy,
)
from aerospike_native.exceptions import AerospikeError, InvalidNamespaceError
from fixtures import (
    AbandonedAwait,
    TestFixtureConnection,
    TestFixtureInsertRecord,
    trace_await,
    wait_for_scan_visible,
)


class TestQuery(TestFixtureInsertRecord):
    """Test client.query() method functionality."""
    bin_name = "bin"

    @pytest.fixture
    def stmt(self):
        """Create a test statement."""
        return Statement("test", "test", [self.bin_name])

    async def test_query_and_recordset(self, client, stmt):
        """Test basic query operation and Recordset functionality."""
        records = await client.query(stmt, PartitionFilter.all(), policy=QueryPolicy())
        assert isinstance(records, Recordset)

        async for record in records:
            assert isinstance(record, Record)

        # Wait for the recordset to become inactive (query finished processing)
        # This ensures the recordset is properly closed after consuming all records
        max_wait = 10  # Maximum 1 second wait
        for _ in range(max_wait):
            if not records.active:
                break
            await asyncio.sleep(0.1)
        
        # Query finished - recordset should be inactive after consuming all records
        assert records.active is False

        # Check that we can call close()
        records.close()

    async def test_with_policy(self, client, stmt):
        """Test query operation with query policy."""
        qp = QueryPolicy()
        records = await client.query(stmt, PartitionFilter.all(), policy=qp)
        assert isinstance(records, Recordset)

    async def test_fail(self, client):
        """Test query operation with invalid parameters raises TypeError."""
        # Test with invalid partition filter type to trigger TypeError
        with pytest.raises(TypeError):
            await client.query(Statement("test", "test", ["bin1"]), "invalid_filter", policy=QueryPolicy())

    async def test_invalid_node_error(self, client):
        """Test query operation with invalid namespace raises InvalidNamespaceError during iteration."""
        stmt_invalid_namespace = Statement("bad_ns", "test", ["bin1"])
        records = await client.query(stmt_invalid_namespace, PartitionFilter.all(), policy=QueryPolicy())

        max_wait = 10
        for _ in range(max_wait):
            if not records.active:
                break
            await asyncio.sleep(0.1)

        with pytest.raises(InvalidNamespaceError):
            async for _ in records:
                pass

    async def test_empty_filters_list_does_not_panic(self, client, stmt):
        """An empty ``filters`` list either runs unfiltered or is refused, never panics."""
        stmt.filters = []
        try:
            records = await client.query(stmt, PartitionFilter.all(), policy=QueryPolicy())
            async for _ in records:
                pass
        except AerospikeError as e:
            assert "panicked" not in str(e)


class TestRecordsetAsyncIteration(TestFixtureConnection):
    """Async iteration over a Recordset: buffered records and loop fairness."""

    ns = "test"
    set_name = "rs_iter"
    total = 2000
    # Long enough for the node streams to fill the recordset's buffer.
    buffer_fill = 0.3

    @pytest.fixture
    async def seeded(self, client):
        wp = WritePolicy()

        # Unbounded, the writes open more connections than a small test
        # server accepts at once.
        in_flight = asyncio.Semaphore(64)

        async def write(i):
            async with in_flight:
                await client.put(Key(self.ns, self.set_name, i), {"n": i}, policy=wp)

        async def write_all():
            await asyncio.gather(*(write(i) for i in range(self.total)))

        await write_all()
        await wait_for_scan_visible(client, self.ns, self.set_name, self.total, write_all)
        return client

    async def _query(self, client, partition_filter=None, policy=None):
        stmt = Statement(self.ns, self.set_name, ["n"])
        return await client.query(
            stmt,
            partition_filter or PartitionFilter.all(),
            policy=policy or QueryPolicy(),
        )

    async def test_buffered_records_resolve_without_suspending(self, seeded):
        records = await self._query(seeded)
        trace = []
        await trace_await(records.__anext__(), trace)
        await asyncio.sleep(self.buffer_fill)

        del trace[:]
        # Few enough steps that the run cannot reach a fairness yield.
        for _ in range(6):
            record = await trace_await(records.__anext__(), trace)
            assert isinstance(record, Record)
        records.close()

        assert trace == [()] * 6

    async def test_full_drain_yields_every_record_once(self, seeded):
        records = await self._query(seeded)
        trace = []
        seen = []
        while True:
            try:
                record = await trace_await(records.__anext__(), trace)
            except StopAsyncIteration:
                break
            seen.append(record.bins["n"])

        assert sorted(seen) == list(range(self.total))
        # A fairness step yields to the loop first and then either has a
        # record ready or waits on the stream like any other step.
        assert set(trace) <= {(), ("future",), ("loop",), ("loop", "future")}

    async def test_buffered_drain_yields_to_other_tasks(self, seeded):
        records = await self._query(seeded)
        await records.__anext__()
        await asyncio.sleep(self.buffer_fill)

        ticks = 0

        async def ticker():
            nonlocal ticks
            while True:
                ticks += 1
                await asyncio.sleep(0)

        task = asyncio.ensure_future(ticker())
        try:
            drained = 0
            async for _ in records:
                drained += 1
                if drained == 650:
                    break
            ticks_during_drain = ticks
        finally:
            task.cancel()
            records.close()

        # The loop gets a turn at least once per 65 records: at most 64
        # buffered records back to back, then one step that yields first.
        assert ticks_during_drain >= 650 // 65

    @pytest.mark.parametrize("pipe_wake", ["0", "1"])
    async def test_fairness_step_delivers_finished_operations(
        self, seeded, monkeypatch, aerospike_host, use_services_alternate, pipe_wake,
    ):
        # A long buffered drain keeps the loop from servicing wakes promptly
        # on either transport, so the fairness step hands this client's
        # finished results over itself.
        monkeypatch.setenv("AEROSPIKE_PIPE_WAKE", pipe_wake)
        cp = ClientPolicy()
        cp.use_services_alternate = use_services_alternate
        client = await new_client(cp, aerospike_host)
        try:
            records = await self._query(client)
            await records.__anext__()
            await asyncio.sleep(self.buffer_fill)

            read = client.get(Key(self.ns, self.set_name, 0), policy=ReadPolicy())
            # Blocks the loop on purpose: the read finishes on its worker
            # while nothing on this thread can resume it.
            time.sleep(0.3)
            assert not read.done()

            fairness_step = None
            for _ in range(65):
                steps = records.__anext__().__await__()
                try:
                    if next(steps) is None:
                        fairness_step = steps
                        break
                except StopIteration:
                    pass
            assert fairness_step is not None
            assert read.done()
            assert read.result().bins["n"] == 0
            records.close()
        finally:
            await client.close()

    async def test_abandoned_fairness_step_loses_no_record(self, seeded):
        records = await self._query(seeded)
        trace = []
        seen = []
        abandoned = 0
        await asyncio.sleep(self.buffer_fill)
        while True:
            try:
                record = await trace_await(
                    records.__anext__(), trace,
                    abandon_on=None if abandoned else "loop",
                )
            except AbandonedAwait:
                abandoned += 1
                continue
            except StopAsyncIteration:
                break
            seen.append(record.bins["n"])

        assert abandoned == 1
        assert sorted(seen) == list(range(self.total))

    async def test_abandoned_stream_wait_loses_at_most_one_record(self, seeded):
        records = await self._query(seeded)
        trace = []
        seen = []
        abandoned = 0
        while True:
            try:
                record = await trace_await(
                    records.__anext__(), trace,
                    abandon_on=None if abandoned else "future",
                )
            except AbandonedAwait:
                abandoned += 1
                continue
            except StopAsyncIteration:
                break
            seen.append(record.bins["n"])
        records.close()

        assert abandoned == 1
        assert len(seen) == len(set(seen))
        # The wait that was dropped may already have taken its record off
        # the stream; nothing else can go missing.
        assert len(set(range(self.total)) - set(seen)) <= 1

    async def test_fairness_step_waits_when_the_stream_runs_dry(self, client):
        # 8 KB records under a rate limit reach the client in bursts of
        # about 127 (one server send buffer) roughly 600 ms apart, so the
        # stream is reliably empty between bursts.
        total = 300
        set_name = "rs_iter_bursts"
        payload = b"x" * 8192
        wp = WritePolicy()

        async def write_all():
            for i in range(total):
                await client.put(Key(self.ns, set_name, i), {"n": i, "p": payload}, policy=wp)

        await write_all()
        await wait_for_scan_visible(client, self.ns, set_name, total, write_all)
        policy = QueryPolicy()
        policy.records_per_second = 200
        stmt = Statement(self.ns, set_name, ["n", "p"])
        records = await client.query(stmt, PartitionFilter.all(), policy=policy)

        seen = [(await records.__anext__()).bins["n"]]
        # Let the rest of the first burst land in the buffer.
        await asyncio.sleep(0.2)
        fairness_step = None
        stream_wait = None
        while stream_wait is None:
            steps = records.__anext__().__await__()
            try:
                suspended_on = next(steps)
            except StopIteration as stop:
                seen.append(stop.value.bins["n"])
                continue
            if suspended_on is None:
                # Keep only the latest fairness step parked: resume the one
                # before it, which either has a record or is the first step
                # to find the stream empty.
                if fairness_step is not None:
                    try:
                        waited_on = next(fairness_step)
                    except StopIteration as stop:
                        seen.append(stop.value.bins["n"])
                    else:
                        stream_wait = (fairness_step, waited_on)
                fairness_step = steps
            else:
                stream_wait = (steps, suspended_on)
        assert fairness_step is not None

        # The first burst is drained and a wait for the second is already
        # queued, so the parked fairness step finds nothing to take.
        fairness_wait = next(fairness_step)
        assert asyncio.isfuture(fairness_wait)

        await asyncio.wait({stream_wait[1], fairness_wait}, timeout=10)
        for steps in (stream_wait[0], fairness_step):
            with pytest.raises(StopIteration) as stop:
                next(steps)
            seen.append(stop.value.value.bins["n"])
        async for record in records:
            seen.append(record.bins["n"])

        assert sorted(seen) == list(range(total))

    async def test_early_close_then_resume_loses_no_record(self, seeded):
        records = await self._query(seeded)
        await records.__anext__()
        await asyncio.sleep(self.buffer_fill)

        seen = []
        async for record in records:
            seen.append(record.bins["n"])
            if len(seen) == 200:
                break
        records.close()
        resume_from = await records.partition_filter()
        assert resume_from is not None

        resumed = await self._query(seeded, partition_filter=resume_from)
        async for record in resumed:
            seen.append(record.bins["n"])

        # The first record was taken and discarded above; a resume never
        # re-delivers what the consumer already took.
        assert len(set(range(self.total)) - set(seen)) <= 1
        assert len(set(seen)) >= self.total - 1

    async def test_concurrent_consumers_share_the_stream(self, seeded):
        records = await self._query(seeded)

        async def consume():
            return [record.bins["n"] async for record in records]

        first, second = await asyncio.gather(consume(), consume())

        assert sorted(first + second) == list(range(self.total))

    async def test_single_consumer_sees_stream_order(
        self, seeded, aerospike_host, use_services_alternate,
    ):
        # Within one partition the server returns records in a fixed order,
        # whichever nodes the partitions come from. Blocking iteration takes
        # every record through a single path, so it is the reference.
        def by_partition(records):
            order = {}
            for record in records:
                order.setdefault(record.key.partition_id, []).append(record.bins["n"])
            return order

        def blocking_pass():
            cp = ClientPolicy()
            cp.use_services_alternate = use_services_alternate
            blocking_client = new_client_blocking(cp, aerospike_host)
            try:
                stmt = Statement(self.ns, self.set_name, ["n"])
                return by_partition(blocking_client.query_blocking(
                    stmt, PartitionFilter.all(), policy=QueryPolicy(),
                ))
            finally:
                blocking_client.close_blocking()

        expected = await asyncio.to_thread(blocking_pass)
        records = await self._query(seeded)
        taken = [await records.__anext__()]
        await asyncio.sleep(self.buffer_fill)
        taken.extend([record async for record in records])
        found = by_partition(taken)

        assert max(len(order) for order in expected.values()) > 1
        assert found == expected

    async def test_empty_result(self, seeded):
        stmt = Statement(self.ns, "rs_iter_no_such_set", [])
        records = await seeded.query(stmt, PartitionFilter.all(), policy=QueryPolicy())

        assert [record async for record in records] == []
        with pytest.raises(StopAsyncIteration):
            await records.__anext__()

    async def test_single_record_result(self, client):
        key = Key(self.ns, "rs_iter_one", 1)

        async def write():
            await client.put(key, {"n": 1}, policy=WritePolicy())

        await write()
        await wait_for_scan_visible(client, self.ns, "rs_iter_one", 1, write)
        stmt = Statement(self.ns, "rs_iter_one", ["n"])
        records = await client.query(stmt, PartitionFilter.all(), policy=QueryPolicy())

        assert [record.bins["n"] async for record in records] == [1]

    async def test_buffered_error_raises_from_the_step(self, client):
        stmt = Statement("bad_ns", "test", ["bin1"])
        records = await client.query(stmt, PartitionFilter.all(), policy=QueryPolicy())
        await asyncio.sleep(self.buffer_fill)

        trace = []
        step = records.__anext__()
        with pytest.raises(InvalidNamespaceError):
            await trace_await(step, trace)

        assert trace == [()]


class TestQueryEmptySet(TestFixtureConnection):
    """Test query with empty set name."""

    async def test_query_empty_set_name_none(self, client):
        """Test query operation with None set name (queries all sets in namespace)."""
        stmt = Statement("test", set_name=None, bins=None)
        qp = QueryPolicy()
        pf = PartitionFilter.all()

        assert stmt.set_name is None

        rs = await client.query(stmt, pf, policy=qp)
        assert isinstance(rs, Recordset)

    async def test_query_empty_set_name_empty_string(self, client):
        """Test query operation with empty string set name (queries all sets in namespace)."""
        stmt = Statement("test", set_name="", bins=None)
        qp = QueryPolicy()
        pf = PartitionFilter.all()

        assert stmt.set_name is None

        rs = await client.query(stmt, pf, policy=qp)
        assert isinstance(rs, Recordset)

    async def test_query_empty_set_name_equivalence(self, client):
        """Test that None and empty string are equivalent for set_name."""
        stmt1 = Statement("test", set_name=None, bins=None)
        stmt2 = Statement("test", set_name="", bins=None)

        # Both should result in None
        assert stmt1.set_name is None
        assert stmt2.set_name is None
        assert stmt1.set_name == stmt2.set_name


class TestQueryEqualByIndex(TestFixtureInsertRecord):
    """Query using a filter that names the secondary index (not Statement.index_name)."""

    idx = "pnc_it_query_equal_by_index"

    async def cleanup_index(self, client):
        try:
            task = await client.drop_index("test", "test", self.idx)
            await task.wait_till_complete()
        except Exception:
            pass

    async def test_query_equal_by_index_returns_record(self, client, wait_for_index):
        await self.cleanup_index(client)
        await client.create_index(
            "test",
            "test",
            "year",
            self.idx,
            IndexType.NUMERIC,
            cit=CollectionIndexType.DEFAULT,
        )
        flt = Filter.equal_by_index(self.idx, 1964)
        await wait_for_index(client, "test", "test", flt, bins=["year"])

        stmt = Statement("test", "test", ["year"])
        stmt.filters = [flt]

        records = await client.query(stmt, PartitionFilter.all(), policy=QueryPolicy())
        found = False
        async for record in records:
            assert isinstance(record, Record)
            assert record.bins.get("year") == 1964
            found = True
        assert found

        await self.cleanup_index(client)


class TestQueryBlobIndex(TestFixtureConnection):
    """Query a bytes bin through a blob secondary index (server 7.0+)."""

    set_name = "blob_idx_set"
    idx_name = "pnc_it_query_blob_equal"
    bin_name = "payload"

    async def cleanup(self, client):
        try:
            await client.truncate("test", self.set_name)
        except Exception:
            pass
        try:
            task = await client.drop_index("test", self.set_name, self.idx_name)
            await task.wait_till_complete()
        except Exception:
            pass

    async def test_query_blob_equal_returns_only_exact_match(self, client, wait_for_index):
        await self.cleanup(client)
        wp = WritePolicy()

        # The decoy shares a prefix with the needle, so a truncating
        # comparison would over-match.
        needle = b"\xde\xad\xbe\xef"
        blobs = [b"\x01\x02", needle, b"\xff", b"\xde\xad"]

        async def write_blobs():
            for i, blob in enumerate(blobs):
                key = Key("test", self.set_name, i)
                await client.put(key, {self.bin_name: blob}, policy=wp)

        await write_blobs()
        # cleanup() truncated this set; a same-clock-tick write is silently
        # expired with the truncate. Verify visibility, re-seeding if eaten.
        await wait_for_scan_visible(
            client, "test", self.set_name, len(blobs), write_blobs)

        task = await client.create_index(
            "test",
            self.set_name,
            self.bin_name,
            self.idx_name,
            IndexType.BLOB,
            cit=CollectionIndexType.DEFAULT,
        )
        await task.wait_till_complete()
        flt = Filter.equal(self.bin_name, needle)
        await wait_for_index(client, "test", self.set_name, flt, bins=[self.bin_name])

        stmt = Statement("test", self.set_name, [self.bin_name])
        stmt.filters = [flt]

        records = await client.query(stmt, PartitionFilter.all(), policy=QueryPolicy())
        matches = []
        async for record in records:
            matches.append(record.bins.get(self.bin_name))
        assert matches == [needle]

        await self.cleanup(client)


class TestQueryFilterContext(TestFixtureConnection):
    """Query with Filter.context for a secondary index on a nested list element."""

    set_name = "flt_ctx_set"
    idx_name = "pnc_it_nested_list_elem"
    bin_name = "nested"

    async def cleanup(self, client):
        try:
            await client.truncate("test", self.set_name)
        except Exception:
            pass
        try:
            task = await client.drop_index("test", self.set_name, self.idx_name)
            await task.wait_till_complete()
        except Exception:
            pass

    async def test_query_list_element_context_filter(self, client, wait_for_index):
        await self.cleanup(client)
        wp = WritePolicy()

        async def write_rows():
            for i in range(5):
                key = Key("test", self.set_name, i)
                await client.put(key, {self.bin_name: [i]}, policy=wp)

        await write_rows()
        # cleanup() truncated this set; a same-clock-tick write is silently
        # expired with the truncate. Verify visibility, re-seeding if eaten.
        await wait_for_scan_visible(
            client, "test", self.set_name, 5, write_rows)

        task = await client.create_index(
            "test",
            self.set_name,
            self.bin_name,
            self.idx_name,
            IndexType.NUMERIC,
            cit=CollectionIndexType.DEFAULT,
            ctx=[CTX.list_index(0)],
        )
        await task.wait_till_complete()
        flt = Filter.range(self.bin_name, 0, 4).context([CTX.list_index(0)])
        await wait_for_index(
            client, "test", self.set_name, flt, bins=[self.bin_name])

        stmt = Statement("test", self.set_name, [self.bin_name])
        stmt.filters = [flt]

        records = await client.query(stmt, PartitionFilter.all(), policy=QueryPolicy())
        count = 0
        async for _ in records:
            count += 1
        assert count == 5

        await self.cleanup(client)


class TestQueryFilterExpressionAttach(TestFixtureInsertRecord):
    """Query using Filter.expression to select an expression-based secondary index."""

    idx = "pnc_it_flt_expr_attach"

    async def cleanup_index(self, client):
        try:
            task = await client.drop_index("test", "test", self.idx)
            await task.wait_till_complete()
        except Exception:
            pass

    async def test_query_range_with_expression_on_filter(self, client, wait_for_index):
        await self.cleanup_index(client)
        expr = FilterExpression.int_bin("year")
        task = await client.create_index_using_expression(
            namespace="test",
            set_name="test",
            index_name=self.idx,
            index_type=IndexType.NUMERIC,
            expression=expr,
        )
        assert await task.wait_till_complete()
        flt = Filter.range("year", 1960, 1970).expression(expr)
        await wait_for_index(client, "test", "test", flt, bins=["year"])

        stmt = Statement("test", "test", ["year"])
        stmt.filters = [flt]

        records = await client.query(stmt, PartitionFilter.all(), policy=QueryPolicy())
        found = False
        async for record in records:
            assert isinstance(record, Record)
            assert record.bins.get("year") == 1964
            found = True
        assert found

        await self.cleanup_index(client)
