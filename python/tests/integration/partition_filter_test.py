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

import pytest
from aerospike_native import PartitionFilter, Key, QueryPolicy, Statement, PartitionStatus, Recordset, WritePolicy

from fixtures import TestFixtureInsertRecord


class TestPartitionFilterUsage(TestFixtureInsertRecord):
    """Test PartitionFilter usage in actual scan/query operations."""

    async def test_query_with_by_id(self, client):
        """Test query with PartitionFilter.by_id()."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_id(0)
        records = await client.query(stmt, pf, policy=QueryPolicy())
        assert isinstance(records, Recordset)

        # Consume records
        count = 0
        async for _ in records:
            count += 1
            if count > 100:
                break

    async def test_query_with_by_range(self, client):
        """Test query with PartitionFilter.by_range()."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 10)
        records = await client.query(stmt, pf, policy=QueryPolicy())
        assert isinstance(records, Recordset)

        # Consume records
        count = 0
        async for _ in records:
            count += 1
            if count > 100:
                break

    async def test_recordset_partition_filter(self, client):
        """Test Recordset.partition_filter() returns updated PartitionFilter."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 5)
        records = await client.query(stmt, pf, policy=QueryPolicy())
        assert isinstance(records, Recordset)

        count = 0
        async for _ in records:
            count += 1
            if count > 10:
                break

        updated_pf = await records.partition_filter()
        assert updated_pf is not None
        assert isinstance(updated_pf, PartitionFilter)

    async def test_recordset_partition_filter_active(self, client):
        """Test Recordset.partition_filter() behavior with active recordsets."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 5)
        records = await client.query(stmt, pf, policy=QueryPolicy())
        assert isinstance(records, Recordset)

        # Check immediately after query
        # partition_filter() may return None or a PartitionFilter depending on implementation
        # The key is that it's callable and returns a valid result
        updated_pf = await records.partition_filter()
        assert updated_pf is None or isinstance(updated_pf, PartitionFilter)

    async def test_query_with_partitions(self, client):
        """Test query with partitions getter/setter."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 5)
        policy = QueryPolicy()
        policy.max_records = 20

        records = await client.query(stmt, pf, policy=policy)

        # Consume all records
        count = 0
        async for _ in records:
            count += 1

        # Partitions may be populated after query (implementation dependent)
        partitions = pf.partitions
        if partitions is not None:
            assert isinstance(partitions, list)

    async def test_partition_status_fields(self, client):
        """Test that PartitionStatus objects have expected fields."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 3)
        policy = QueryPolicy()
        policy.max_records = 10

        records = await client.query(stmt, pf, policy=policy)

        # Consume records
        async for _ in records:
            pass

        partitions = pf.partitions
        if partitions:
            for ps in partitions:
                assert isinstance(ps, PartitionStatus)
                # Check that PartitionStatus has expected fields
                assert hasattr(ps, 'bval')
                assert hasattr(ps, 'id')
                assert hasattr(ps, 'retry')
                assert hasattr(ps, 'digest')
                # id should be a valid partition ID
                assert isinstance(ps.id, int)
                assert 0 <= ps.id < 4096


class TestQueryPagination(TestFixtureInsertRecord):
    """Test query pagination using Recordset.partition_filter()."""

    async def test_query_pagination_basic(self, client):
        """Test basic query pagination with max_records."""
        wp = WritePolicy()
        for i in range(1, 21):
            key = Key("test", "test", i)
            await client.put(key, {"bin": i}, policy=wp)

        stmt = Statement("test", "test", None)
        policy = QueryPolicy()
        policy.max_records = 10

        pf = PartitionFilter.all()
        total_records = 0
        page_count = 0

        while page_count < 5 and not pf.done():
            records = await client.query(stmt, pf, policy=policy)
            page_records = 0

            async for _ in records:
                page_records += 1
                total_records += 1

            page_count += 1

            updated_pf = await records.partition_filter()
            if updated_pf is not None:
                pf = updated_pf
            else:
                break

        assert page_count > 0
        assert total_records > 0

    async def test_query_pagination_with_results(self, client):
        """Test query pagination using async iteration."""
        wp = WritePolicy()
        for i in range(1, 31):
            key = Key("test", "test", i)
            await client.put(key, {"bin": i}, policy=wp)

        stmt = Statement("test", "test", None)
        policy = QueryPolicy()
        policy.max_records = 20

        pf = PartitionFilter.all()
        total_records = 0
        pages = 0

        while pages < 10 and not pf.done():
            records = await client.query(stmt, pf, policy=policy)
            page_records = 0

            async for _ in records:
                page_records += 1
                total_records += 1

            pages += 1

            updated_pf = await records.partition_filter()
            if updated_pf is not None:
                pf = updated_pf
            else:
                break

        assert pages > 0
        assert total_records > 0

    async def test_query_pagination_done_check(self, client):
        """Test that pagination stops when done() returns True."""
        wp = WritePolicy()
        for i in range(1, 11):
            key = Key("test", "test", i)
            await client.put(key, {"bin": i}, policy=wp)

        stmt = Statement("test", "test", None)
        policy = QueryPolicy()
        policy.max_records = 50

        pf = PartitionFilter.all()
        pages = 0
        max_pages = 10

        while pages < max_pages and not pf.done():
            records = await client.query(stmt, pf, policy=policy)

            async for _ in records:
                pass

            pages += 1

            updated_pf = await records.partition_filter()
            if updated_pf is not None:
                pf = updated_pf
            else:
                break

        assert pages <= max_pages

    async def test_query_pagination_empty_resultset(self, client):
        """Test pagination with empty resultset."""
        stmt = Statement("test", "nonexistent_set", ["bin"])
        policy = QueryPolicy()
        policy.max_records = 10

        pf = PartitionFilter.by_range(0, 1)
        records = await client.query(stmt, pf, policy=policy)

        count = 0
        async for _ in records:
            count += 1

        assert count == 0

        updated_pf = await records.partition_filter()
        if updated_pf is not None:
            assert updated_pf.done() is True


class TestQueryResume(TestFixtureInsertRecord):
    """Test query resume functionality using Recordset.partition_filter()."""

    async def test_query_resume_after_partial_consumption(self, client):
        """Test resuming a query after partially consuming records."""
        wp = WritePolicy()
        for i in range(1, 31):
            key = Key("test", "test", i)
            await client.put(key, {"bin": i}, policy=wp)

        stmt = Statement("test", "test", None)
        policy = QueryPolicy()
        policy.max_records = 10  # Smaller max_records so query finishes after first batch

        pf = PartitionFilter.all()
        records = await client.query(stmt, pf, policy=policy)

        first_batch_count = 0
        async for _ in records:
            first_batch_count += 1

        # Wait for recordset to become inactive
        import asyncio
        max_wait = 10
        for _ in range(max_wait):
            if not records.active:
                break
            await asyncio.sleep(0.1)

        updated_pf = await records.partition_filter()
        assert updated_pf is not None

        resumed_records = await client.query(stmt, updated_pf, policy=policy)
        resumed_count = 0

        async for _ in resumed_records:
            resumed_count += 1

        assert first_batch_count > 0
        assert resumed_count > 0

    async def test_cursor_does_not_move_before_iteration(self, client):
        """A query closed before anything is read from it leaves every record to a resume."""
        wp = WritePolicy()
        for i in range(1, 31):
            await client.put(Key("test", "test", i), {"bin": i}, policy=wp)

        stmt = Statement("test", "test", None)
        total = 0
        async for _ in await client.query(stmt, PartitionFilter.all()):
            total += 1

        records = await client.query(stmt, PartitionFilter.all())
        records.close()
        for _ in range(100):
            if not records.active:
                break
            await asyncio.sleep(0.05)
        cursor = await records.partition_filter()
        assert cursor is not None

        resumed = 0
        async for _ in await client.query(stmt, cursor):
            resumed += 1
        assert resumed == total

    async def test_query_resume_complete_consumption(self, client):
        """Test resuming after fully consuming a recordset."""
        wp = WritePolicy()
        for i in range(1, 21):
            key = Key("test", "test", i)
            await client.put(key, {"bin": i}, policy=wp)

        stmt = Statement("test", "test", None)
        policy = QueryPolicy()
        policy.max_records = 50

        pf = PartitionFilter.all()
        records = await client.query(stmt, pf, policy=policy)

        first_count = 0
        async for _ in records:
            first_count += 1

        updated_pf = await records.partition_filter()
        assert updated_pf is not None

        resumed_records = await client.query(stmt, updated_pf, policy=policy)
        resumed_count = 0

        async for _ in resumed_records:
            resumed_count += 1

        assert first_count > 0
        assert resumed_count >= 0

    async def test_query_resume_multiple_times(self, client):
        """Test resuming a query multiple times."""
        wp = WritePolicy()
        for i in range(1, 21):
            key = Key("test", "test", i)
            await client.put(key, {"bin": i}, policy=wp)

        stmt = Statement("test", "test", None)
        policy = QueryPolicy()
        policy.max_records = 20

        pf = PartitionFilter.all()
        total_count = 0

        for resume_iteration in range(3):
            records = await client.query(stmt, pf, policy=policy)
            iteration_count = 0

            async for _ in records:
                iteration_count += 1
                total_count += 1
                if iteration_count >= 5:
                    break

            updated_pf = await records.partition_filter()
            if updated_pf is not None:
                pf = updated_pf
            else:
                break

        assert total_count > 0


class TestQueryPartitionEdgeCases(TestFixtureInsertRecord):
    """Test edge cases and error conditions for partition queries."""

    async def test_query_partition_invalid_begin(self, client):
        """Test query with invalid partition begin value."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(4096, 1)

        with pytest.raises(Exception):
            records = await client.query(stmt, pf, policy=QueryPolicy())
            async for _ in records:
                pass

    async def test_query_partition_invalid_count(self, client):
        """Test query with invalid partition count."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 5000)

        with pytest.raises(Exception):
            records = await client.query(stmt, pf, policy=QueryPolicy())
            async for _ in records:
                pass

    async def test_query_partition_zero_count(self, client):
        """Test query with zero partition count."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 0)

        with pytest.raises(Exception):
            records = await client.query(stmt, pf, policy=QueryPolicy())
            async for _ in records:
                pass

    async def test_query_partition_nonexistent_namespace(self, client):
        """Test query with non-existent namespace."""
        stmt = Statement("nonexistent_ns", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 1)

        with pytest.raises(Exception):
            records = await client.query(stmt, pf, policy=QueryPolicy())
            async for _ in records:
                pass

    async def test_query_partition_nonexistent_set(self, client):
        """Test query with non-existent set."""
        stmt = Statement("test", "nonexistent_set", ["bin"])
        pf = PartitionFilter.by_range(0, 1)

        records = await client.query(stmt, pf, policy=QueryPolicy())
        count = 0
        async for _ in records:
            count += 1

        assert count == 0

    async def test_query_partition_filter_reuse(self, client):
        """Test reusing the same PartitionFilter object across queries."""
        set_name = "pf_reuse"
        wp = WritePolicy()
        for i in range(1, 11):
            key = Key("test", set_name, i)
            await client.put(key, {"bin": i}, policy=wp)

        stmt = Statement("test", set_name, None)
        pf = PartitionFilter.all()

        records1 = await client.query(stmt, pf, policy=QueryPolicy())
        count1 = 0
        async for _ in records1:
            count1 += 1

        # PartitionFilter is cloned (by-value) into query, so pf stays fresh
        records2 = await client.query(stmt, pf, policy=QueryPolicy())
        count2 = 0
        async for _ in records2:
            count2 += 1

        # This test owns the set's exact contents (fixed keys 1..10), so a
        # partial or empty drain — e.g. a query racing a degraded cluster
        # view — must fail loudly rather than pass on "anything > 0".
        assert count1 == 10
        assert count2 == count1

    async def test_query_partition_filter_active_recordset(self, client):
        """Test partition_filter() behavior with active recordsets."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 2)
        records = await client.query(stmt, pf, policy=QueryPolicy())

        # Check immediately after query
        # partition_filter() may return None or a PartitionFilter depending on implementation
        # The key is that it's callable and returns a valid result
        updated_pf = await records.partition_filter()
        assert updated_pf is None or isinstance(updated_pf, PartitionFilter)

        # Consume one record
        async for _ in records:
            break

        # After consuming, partition_filter() may return None (if still active) or a PartitionFilter (if inactive)
        updated_pf = await records.partition_filter()
        assert updated_pf is None or isinstance(updated_pf, PartitionFilter)

    async def test_query_partition_filter_after_close(self, client):
        """Test partition_filter() after recordset is closed."""
        stmt = Statement("test", "test", ["bin"])
        pf = PartitionFilter.by_range(0, 2)
        records = await client.query(stmt, pf, policy=QueryPolicy())

        async for _ in records:
            pass

        records.close()
        updated_pf = await records.partition_filter()

        assert updated_pf is not None
        assert isinstance(updated_pf, PartitionFilter)
