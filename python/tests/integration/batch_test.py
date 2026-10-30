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

import base64
import os
import pytest
import pytest_asyncio

from aerospike_native import (
    new_client, ClientPolicy, WritePolicy, ReadPolicy, Key,
    BatchPolicy, BatchReadPolicy, BatchWritePolicy, BatchDeletePolicy, BatchRecord, ListOperation, Operation, ListReturnType,
    FilterExpression, ListPolicy, Expiration,
    ExpOperation, ExpReadFlags,
    BatchReadOp, BatchWriteOp, BatchDeleteOp,
    RecordExistsAction,
    MapOperation, MapPolicy, MapReturnType, CTX,
)
from aerospike_native.exceptions import ServerError, ResultCode, InvalidNamespaceError, RecordNotFound

@pytest_asyncio.fixture
async def client_and_keys():
    """Setup client and create test records for batch operations."""

    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, os.environ.get("AEROSPIKE_HOST", "localhost:3000"))

    wp = WritePolicy()
    size = 8
    keys = []
    bin_name = "bbin"

    for i in range(1, size + 1):
        key = Key("test", "test", f"batchkey{i}")
        keys.append(key)
        # List contains [0*i, 1*i, 2*i, ..., (i-1)*i]
        list_bin = [j * i for j in range(i)]
        # list_bin2 is always [0, 1] for all keys
        list_bin2 = [0, 1]
        if i != 6:
            await client.put(key, {bin_name: f"batchvalue{i}", "lbin": list_bin, "lbin2": list_bin2}, policy=wp)
        else:
            await client.put(key, {bin_name: i, "lbin": list_bin, "lbin2": list_bin2}, policy=wp)

    delete_keys = [
        Key("test", "test", 10000),
        Key("test", "test", 10001),
        Key("test", "test", 10002),
    ]
    for key in delete_keys:
        await client.put(key, {bin_name: key.value}, policy=wp)

    yield client, keys, delete_keys, bin_name
    await client.close()

async def test_batch_read(client_and_keys):
    """Test batch read operations."""

    client, keys, _, bin_name = client_and_keys

    results = await client.batch_read(keys, [bin_name], batch_policy=None, read_policy=None)

    assert len(results) == len(keys)

    for i, result in enumerate(results):
        assert result.result_code is not None
        assert result.result_code == ResultCode.OK
        assert result.record is not None
        if i != 5:
            assert result.record.bins[bin_name] == f"batchvalue{i + 1}"
        else:
            assert result.record.bins[bin_name] == i + 1

async def test_batch_record_returns_the_same_record(client_and_keys):
    """Every read of ``record`` returns one object whose bins are built once."""

    client, keys, _, bin_name = client_and_keys

    results = await client.batch_read(keys[:1], [bin_name], batch_policy=None, read_policy=None)

    br = results[0]
    assert br.record is br.record
    assert br.record.bins is br.record.bins
    assert br.record.bins[bin_name] == "batchvalue1"
    assert br.result_code == ResultCode.OK

async def test_batch_read_all_bins(client_and_keys):
    """Test batch read with all bins.

    Note: Rust core doesn't support Operation.get() in batch write, so we:
    1. First write the bin using batch_operate
    2. Then read all bins using batch_read with bins=None
    """

    client, keys, _, bin_name = client_and_keys

    operations = [
        Operation.put("bin5", "NewValue")
    ]

    write_results = await client.batch_operate(keys, [operations] * len(keys), batch_policy=None, write_policy=None)

    assert len(write_results) == len(keys)
    for result in write_results:
        assert result.result_code == ResultCode.OK

    read_results = await client.batch_read(keys, None, batch_policy=None, read_policy=None)

    assert len(read_results) == len(keys)

    for i, result in enumerate(read_results):
        assert result.result_code is not None
        assert result.result_code == ResultCode.OK
        assert result.record is not None

        s = result.record.bins.get("bin5")
        assert s == "NewValue"

        obj = result.record.bins.get(bin_name)
        assert obj is not None

async def test_batch_read_empty_bins(client_and_keys):
    """Test batch read with empty bin names list.

    Note: Empty bin names array returns all bins.
    We use None (which reads all bins) instead of [].
    """

    client, keys, _, bin_name = client_and_keys

    results = await client.batch_read(keys, None, batch_policy=None, read_policy=None)

    assert len(results) == len(keys)

    for i, result in enumerate(results):
        assert result.result_code is not None
        assert result.result_code == ResultCode.OK
        assert result.record is not None
        if i != 5:
            assert result.record.bins[bin_name] == f"batchvalue{i + 1}"
        else:
            assert result.record.bins[bin_name] == i + 1

async def test_batch_read_complex(client_and_keys):
    """Test complex batch read scenarios.

    This test covers:
    - Specific bin names
    - All bins (None)
    - Empty bin list (headers only)
    - Non-existent bin
    - Non-existent key
    """

    client, keys, _, bin_name = client_and_keys

    results1 = await client.batch_read([keys[0]], [bin_name], batch_policy=None, read_policy=None)
    assert results1[0].result_code == ResultCode.OK
    assert results1[0].record.bins[bin_name] == "batchvalue1"

    results2 = await client.batch_read([keys[1]], None, batch_policy=None, read_policy=None)
    assert results2[0].result_code == ResultCode.OK
    assert bin_name in results2[0].record.bins

    results3 = await client.batch_read([keys[2]], [], batch_policy=None, read_policy=None)
    assert results3[0].result_code == ResultCode.OK
    assert len(results3[0].record.bins) == 0
    assert results3[0].record.generation > 0

    results4 = await client.batch_read([keys[3]], [bin_name], batch_policy=None, read_policy=None)
    assert results4[0].result_code == ResultCode.OK
    assert bin_name in results4[0].record.bins

    results5 = await client.batch_read([keys[4]], None, batch_policy=None, read_policy=None)
    assert results5[0].result_code == ResultCode.OK
    assert bin_name in results5[0].record.bins

    results6 = await client.batch_read([keys[6]], [bin_name], batch_policy=None, read_policy=None)
    assert results6[0].result_code == ResultCode.OK
    assert results6[0].record.bins[bin_name] == "batchvalue7"

    results7 = await client.batch_read([keys[7]], ["binnotfound"], batch_policy=None, read_policy=None)
    assert results7[0].result_code == ResultCode.OK
    assert "binnotfound" not in results7[0].record.bins

    non_existent_key = Key("test", "test", "keynotfound")
    results8 = await client.batch_read([non_existent_key], [bin_name], batch_policy=None, read_policy=None)
    assert results8[0].result_code == ResultCode.KEY_NOT_FOUND_ERROR
    assert results8[0].record is None

async def test_batch_read_key_not_found(client_and_keys):
    """Test batch read with non-existent key (simplified from batchReadComplex)."""

    client, _, _, bin_name = client_and_keys

    non_existent_key = Key("test", "test", "keynotfound")
    results = await client.batch_read([non_existent_key], [bin_name], batch_policy=None, read_policy=None)

    assert len(results) == 1
    assert results[0].result_code is not None
    assert results[0].result_code == ResultCode.KEY_NOT_FOUND_ERROR
    assert results[0].record is None

async def test_batch_write(client_and_keys):
    """Test batch write operations."""

    client, keys, _, _ = client_and_keys

    bp = BatchPolicy()
    bwp = BatchWritePolicy()
    bins_list = [{"newbin": f"newvalue{i}"} for i in range(len(keys))]

    results = await client.batch_write(keys, bins_list, batch_policy=bp, write_policy=bwp)

    assert len(results) == len(keys)

    for result in results:
        assert result.result_code is not None
        assert result.result_code == ResultCode.OK

    rp = ReadPolicy()
    for i, key in enumerate(keys):
        rec = await client.get(key, policy=rp)
        assert rec is not None
        assert rec.bins["newbin"] == f"newvalue{i}"

async def test_batch_delete(client_and_keys):
    """Test batch delete operations.

    Note: The Rust core doesn't provide a status field, so we verify individual result codes
    and existence checks instead.
    """

    client, _, delete_keys, _ = client_and_keys

    rp = ReadPolicy()
    for key in delete_keys[:2]:
        rec = await client.get(key, policy=rp)
        assert rec is not None

    results = await client.batch_delete(delete_keys[:2], batch_policy=None, delete_policy=None)

    assert len(results) == 2
    assert results[0].result_code == ResultCode.OK
    assert results[1].result_code == ResultCode.OK

    for key in delete_keys[:2]:
        try:
            rec = await client.get(key, policy=rp)
            assert rec is None
        except ServerError as e:
            assert e.result_code == ResultCode.KEY_NOT_FOUND_ERROR
            assert isinstance(e, RecordNotFound)

async def test_batch_delete_key_not_found(client_and_keys):
    """Test batch delete with non-existent key.

    Note: The Rust core doesn't provide a status field, so we verify the result code instead.
    """

    client, _, _, _ = client_and_keys

    non_existent_key = Key("test", "test", 989299023)
    results = await client.batch_delete([non_existent_key], batch_policy=None, delete_policy=None)

    assert len(results) == 1
    assert results[0].result_code is not None
    assert results[0].result_code == ResultCode.KEY_NOT_FOUND_ERROR

async def test_batch_read_multiple_bins(client_and_keys):
    """Test batch read with multiple bin names.

    Note: Python-specific test. Verifies that batch_read correctly filters
    bins when multiple bin names are specified.
    """

    client, keys, _, _ = client_and_keys

    wp = WritePolicy()
    await client.put(keys[0], {"bin1": "value1", "bin2": "value2", "bin3": "value3"}, policy=wp)

    bp = BatchPolicy()
    brp = BatchReadPolicy()
    results = await client.batch_read([keys[0]], ["bin1", "bin2"], batch_policy=bp, read_policy=brp)

    assert len(results) == 1
    assert results[0].result_code is not None
    assert results[0].result_code == ResultCode.OK
    assert results[0].record is not None
    assert "bin1" in results[0].record.bins
    assert "bin2" in results[0].record.bins
    assert "bin3" not in results[0].record.bins

async def test_batch_write_empty_keys(client_and_keys):
    """Test batch write with empty keys list.

    Note: Python-specific edge case test. Verifies that empty key lists
    are handled correctly without errors.
    """

    client, _, _, _ = client_and_keys

    bp = BatchPolicy()
    bwp = BatchWritePolicy()
    results = await client.batch_write([], [], batch_policy=bp, write_policy=bwp)

    assert len(results) == 0

async def test_batch_read_empty_keys(client_and_keys):
    """Test batch read with empty keys list.

    Note: Python-specific edge case test. Verifies that empty key lists
    are handled correctly without errors.
    """

    client, _, _, _ = client_and_keys

    bp = BatchPolicy()
    brp = BatchReadPolicy()
    results = await client.batch_read([], ["bin"], batch_policy=bp, read_policy=brp)

    assert len(results) == 0

async def test_batch_write_different_bins_per_key(client_and_keys):
    """Test batch write with different bins for each key.

    Note: Python-specific test. Verifies that batch_write correctly handles
    different bin data for each key in the batch.
    """

    client, keys, _, _ = client_and_keys

    bp = BatchPolicy()
    bwp = BatchWritePolicy()
    bins_list = [
        {"a": 1, "b": "first"},
        {"a": 2, "b": "second"},
        {"a": 3, "b": "third"},
    ]

    results = await client.batch_write(keys[:3], bins_list, batch_policy=bp, write_policy=bwp)

    assert len(results) == 3
    for result in results:
        assert result.result_code is not None
        assert result.result_code == ResultCode.OK

    rp = ReadPolicy()
    for i, key in enumerate(keys[:3]):
        rec = await client.get(key, policy=rp)
        assert rec is not None
        assert rec.bins["a"] == i + 1
        assert rec.bins["b"] == ["first", "second", "third"][i]

async def test_batch_record_properties(client_and_keys):
    """Test BatchRecord properties.

    Note: Python-specific test. Verifies that BatchRecord objects have
    all expected properties populated correctly.
    """

    client, keys, _, bin_name = client_and_keys

    bp = BatchPolicy()
    brp = BatchReadPolicy()
    results = await client.batch_read([keys[0]], [bin_name], batch_policy=bp, read_policy=brp)

    assert len(results) == 1
    record = results[0]

    assert isinstance(record, BatchRecord)
    assert record.key is not None
    assert record.record is not None
    assert record.result_code is not None
    assert record.result_code == ResultCode.OK
    assert record.in_doubt is False

async def test_batch_read_with_filter_expression(client_and_keys):
    """Test batch read with filter expression.

    Note: Python-specific test. Verifies that filter expressions work
    correctly with batch read operations.
    """

    client, keys, _, bin_name = client_and_keys

    wp = WritePolicy()
    await client.put(keys[0], {bin_name: "match"}, policy=wp)

    bp = BatchPolicy()
    brp = BatchReadPolicy()
    brp.filter_expression = FilterExpression.eq(FilterExpression.string_bin(bin_name), FilterExpression.string_val("match"))

    results = await client.batch_read([keys[0]], [bin_name], batch_policy=bp, read_policy=brp)

    assert len(results) == 1
    assert results[0].result_code is not None
    assert results[0].result_code == ResultCode.OK
    assert results[0].record is not None
    assert results[0].record.bins[bin_name] == "match"


async def test_batch_read_invalid_filter_expression_bytes_returns_parameter_error():
    """Decodable base64 that is not valid expression wire data yields PARAMETER_ERROR per record.

    ``FilterExpression.from_base64`` stores raw bytes; the server validates on execute.
    """
    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, os.environ.get("AEROSPIKE_HOST", "localhost:3000"))
    wp = WritePolicy()
    key = Key("test", "test", "batch_bad_filter_expr_key")
    try:
        await client.delete(key, policy=wp)
        await client.put(key, {"b": 1}, policy=wp)
        garbage_b64 = base64.b64encode(b"\xff\x00not-a-valid-expression").decode("ascii")
        bad_expr = FilterExpression.from_base64(garbage_b64)
        brp = BatchReadPolicy()
        brp.filter_expression = bad_expr
        results = await client.batch_read([key], ["b"], batch_policy=None, read_policy=brp)
        assert len(results) == 1
        assert results[0].result_code == ResultCode.PARAMETER_ERROR
    finally:
        try:
            await client.delete(key, policy=wp)
        except Exception:
            pass
        await client.close()


async def test_batch_write_with_policy(client_and_keys):
    """Test batch write with custom write policy.

    Note: Python-specific test. Verifies that custom BatchWritePolicy
    settings (like send_key) are applied correctly.
    """

    client, keys, _, _ = client_and_keys

    bp = BatchPolicy()
    bwp = BatchWritePolicy()
    bwp.send_key = True
    bwp.durable_delete = False

    bins_list = [{"testbin": "testvalue"} for _ in keys[:2]]
    results = await client.batch_write(keys[:2], bins_list, batch_policy=bp, write_policy=bwp)

    assert len(results) == 2
    for result in results:
        assert result.result_code is not None
        assert result.result_code == ResultCode.OK

async def test_batch_mixed_operations(client_and_keys):
    """Test that we can perform multiple batch operations in sequence.

    Note: Python-specific test. Verifies that different batch operation
    types (read, write, delete) can be called in sequence without issues.
    """

    client, keys, delete_keys, bin_name = client_and_keys

    bp = BatchPolicy()
    brp = BatchReadPolicy()
    bwp = BatchWritePolicy()
    bdp = BatchDeletePolicy()

    read_results = await client.batch_read(keys[:3], [bin_name], batch_policy=bp, read_policy=brp)
    assert len(read_results) == 3

    bins_list = [{"new": "data"} for _ in keys[:3]]
    write_results = await client.batch_write(keys[:3], bins_list, batch_policy=bp, write_policy=bwp)
    assert len(write_results) == 3

    delete_results = await client.batch_delete(delete_keys[:1], batch_policy=bp, delete_policy=bdp)
    assert len(delete_results) == 1


async def test_batch_read_headers(client_and_keys):
    """Test batch read headers only."""

    client, keys, _, _ = client_and_keys

    results = await client.batch_get_header(keys, batch_policy=None, read_policy=None)

    assert len(results) == len(keys)

    for i, record in enumerate(results):
        assert record is not None
        assert record.bins == {}
        assert record.generation is not None
        assert record.generation > 0
        if record.ttl is not None and record.ttl > 0:
            assert record.ttl > 0

async def test_batch_list_read_operate(client_and_keys):
    """Test batch read with list operations.

    Note: The Rust core returns all operation results as a list when multiple
    operations target the same bin (e.g., record.bins["lbin"] -> [size, value]).
    This test verifies that all operation results are returned.
    """

    client, keys, _, _ = client_and_keys

    operations = [
        ListOperation.size("lbin"),
        ListOperation.get_by_index("lbin", -1, ListReturnType.VALUE)
    ]

    results = await client.batch_operate(keys, [operations] * len(keys), batch_policy=None, write_policy=None)

    assert len(results) == len(keys)

    for i, result in enumerate(results):
        assert result.result_code is not None
        assert result.result_code == ResultCode.OK
        assert result.record is not None
        val = result.record.bins["lbin"]

        assert isinstance(val, list)
        assert len(val) == 2
        # enumerate index i corresponds to key "batchkey{i+1}" which has list [0*(i+1), 1*(i+1), 2*(i+1), ..., (i)*(i+1)]
        # So size = i+1, and last value = i * (i+1)
        assert val[0] == i + 1  # size: list has i+1 elements
        assert val[1] == i * (i + 1)  # value: last element in list [0*(i+1), 1*(i+1), ..., i*(i+1)] is i*(i+1)

async def test_batch_list_write_operate(client_and_keys):
    """Test batch write with list operations.

    Note: The Rust core returns all operation results as a list when multiple
    operations target the same bin (e.g., record.bins["lbin2"] -> [insert_result, size, value]).
    This test verifies that all operation results are returned.
    """

    client, keys, _, _ = client_and_keys

    # lbin2 is already set up in the fixture as [0, 1] for all keys
    list_policy = ListPolicy(None, None)
    operations = [
        ListOperation.insert("lbin2", 0, 1000, list_policy),
        ListOperation.size("lbin2"),
        ListOperation.get_by_index("lbin2", -1, ListReturnType.VALUE)
    ]

    results = await client.batch_operate(keys, [operations] * len(keys), batch_policy=None, write_policy=None)

    assert len(results) == len(keys)

    for result in results:
        assert result.result_code is not None
        assert result.result_code == ResultCode.OK
        assert result.record is not None
        val = result.record.bins["lbin2"]

        assert isinstance(val, list)
        assert len(val) == 3
        assert val[0] == 3  # insert_result (new size after insert)
        assert val[1] == 3  # size after insert
        assert val[2] == 1  # value from get_by_index(-1)

async def test_batch_operate_complex(client_and_keys):
    """Test complex batch operate with mixed operations.

    Note: Invalid namespace handling is tested separately in test_batch_invalid_namespace.
    Batch delete requires a separate batch_delete call.
    The Rust core doesn't provide a status field (indicates if all records succeeded).
    """

    client, keys, delete_keys, bin_name = client_and_keys

    bwp = BatchWritePolicy()
    bwp.send_key = True

    wops1 = [Operation.put("bbin2", 100)]
    wops2 = [Operation.put("bbin3", 200)]

    operations_list = [
        wops1,  # Key 1: write bbin2 = 100
        wops2,  # Key 6: write bbin3 = 200
    ]

    results = await client.batch_operate([keys[0], keys[5]], operations_list, batch_policy=None, write_policy=bwp)

    assert len(results) == 2
    assert results[0].result_code == ResultCode.OK
    assert results[1].result_code == ResultCode.OK

    delete_results = await client.batch_delete([delete_keys[2]], batch_policy=None, delete_policy=None)

    assert len(delete_results) == 1
    assert delete_results[0].result_code == ResultCode.OK

    read_results = await client.batch_read([keys[0], keys[5], delete_keys[2]], ["bbin2", "bbin3"], batch_policy=None, read_policy=None)

    assert len(read_results) == 3
    assert read_results[0].result_code == ResultCode.OK
    assert read_results[0].record.bins["bbin2"] == 100
    assert read_results[1].result_code == ResultCode.OK
    assert read_results[1].record.bins["bbin3"] == 200
    assert read_results[2].result_code == ResultCode.KEY_NOT_FOUND_ERROR

async def test_batch_invalid_namespace(client_and_keys):
    """Test batch operations with invalid namespace.

    When every key is unroutable the Rust core still fails the call outright
    (no batch left to send). Mixed batches record per-key INVALID_NAMESPACE
    instead; see ``test_batch_mixed_with_invalid_namespace``.
    """

    client, keys, _, _ = client_and_keys

    invalid_key = Key("invalid", "test", "batchkey1")

    with pytest.raises(InvalidNamespaceError):
        await client.batch_read([invalid_key], ["bbin"], batch_policy=None, read_policy=None)

    bwp = BatchWritePolicy()
    with pytest.raises(InvalidNamespaceError):
        await client.batch_write([invalid_key], [{"bbin": 100}], batch_policy=None, write_policy=bwp)

    operations = [Operation.put("bbin", 100)]
    with pytest.raises(InvalidNamespaceError):
        await client.batch_operate([invalid_key], [operations], batch_policy=None, write_policy=bwp)

async def test_batch_exists(client_and_keys):
    """Test batch exists operations."""

    client, keys, _, _ = client_and_keys

    exists_results = await client.batch_exists(keys, batch_policy=None, read_policy=None)

    assert len(exists_results) == len(keys)
    for exists in exists_results:
        assert exists is True

@pytest.mark.slow
async def test_batch_read_ttl(client_and_keys):
    """Test batch read with TTL expiration.

    Note: This test sleeps ~11 seconds total waiting for TTL expiration.
    Marked with @pytest.mark.slow so it can be excluded with: pytest -m "not slow"

    TTL must be enabled on the Aerospike server. To enable TTL, configure the server with:
    - namespace <namespace> { default-ttl 30D; nsup-period 120; }
    See: https://aerospike.com/docs/database/manage/namespace/retention/
    """

    import asyncio

    client, _, _, _ = client_and_keys

    key1 = Key("test", "test", 88888)
    key2 = Key("test", "test", 88889)

    # Check if TTL is supported by trying to set expiration and write, then verify TTL is actually set
    bwp = BatchWritePolicy()
    try:
        bwp.expiration = Expiration.seconds(5)
        operations = [Operation.put("a", 1)]
        await client.batch_operate([key1], [operations], batch_policy=None, write_policy=bwp)
        rp = ReadPolicy()
        test_rec = await client.get(key1, policy=rp)
        if test_rec is None or test_rec.ttl is None or test_rec.ttl == 0:
            pytest.skip("TTL not enabled on this server (expiration was set but record has no TTL)")
    except Exception as e:
        pytest.skip(f"TTL not supported on this server: {e}")

    # Write both keys with 5s TTL
    operations = [Operation.put("a", 1)]
    await client.batch_operate([key1, key2], [operations, operations], batch_policy=None, write_policy=bwp)

    # Wait 3s (both keys still alive within 5s TTL)
    await asyncio.sleep(3)

    brp1 = BatchReadPolicy()
    brp1.read_touch_ttl = 80  # refreshes key1's TTL

    brp2 = BatchReadPolicy()
    brp2.read_touch_ttl = -1  # no refresh for key2

    bp = BatchPolicy()
    results1 = await client.batch_read([key1], ["a"], batch_policy=bp, read_policy=brp1)
    results2 = await client.batch_read([key2], ["a"], batch_policy=bp, read_policy=brp2)

    assert results1[0].result_code == ResultCode.OK
    assert results2[0].result_code == ResultCode.OK

    # Wait 3s — key2 has now exceeded its 5s TTL, key1 was refreshed
    await asyncio.sleep(3)

    brp1.read_touch_ttl = -1
    brp2.read_touch_ttl = -1

    results1 = await client.batch_read([key1], ["a"], batch_policy=bp, read_policy=brp1)
    results2 = await client.batch_read([key2], ["a"], batch_policy=bp, read_policy=brp2)

    assert results1[0].result_code == ResultCode.OK
    assert results2[0].result_code == ResultCode.KEY_NOT_FOUND_ERROR

    # Wait 5s — key1's TTL was reset to 5s at t=3s, so it expires at t=8s.
    # At t=11s both keys should be expired.
    await asyncio.sleep(5)

    results1 = await client.batch_read([key1], ["a"], batch_policy=bp, read_policy=brp1)
    results2 = await client.batch_read([key2], ["a"], batch_policy=bp, read_policy=brp2)

    assert results1[0].result_code == ResultCode.KEY_NOT_FOUND_ERROR
    assert results2[0].result_code == ResultCode.KEY_NOT_FOUND_ERROR


@pytest_asyncio.fixture
async def exp_client_and_keys():
    """Setup client and create test records for batch expression operation tests."""
    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, os.environ.get("AEROSPIKE_HOST", "localhost:3000"))

    wp = WritePolicy()
    keys = [
        Key("test", "test", "batchexp1"),
        Key("test", "test", "batchexp2"),
        Key("test", "test", "batchexp3"),
    ]
    for i, key in enumerate(keys, start=1):
        await client.put(key, {"A": i * 10, "B": i * 100}, policy=wp)

    yield client, keys
    for key in keys:
        try:
            await client.delete(key, policy=wp)
        except ServerError:
            pass
    await client.close()


async def test_batch_operate_exp_read(exp_client_and_keys):
    """Test batch_operate with ExpOperation.read across multiple keys."""
    client, keys = exp_client_and_keys

    expr = FilterExpression.num_add([FilterExpression.int_bin("A"), FilterExpression.int_val(5)])
    ops = [ExpOperation.read("result", expr)]

    results = await client.batch_operate(keys, [ops] * len(keys), batch_policy=None, write_policy=None)

    assert len(results) == len(keys)
    for i, result in enumerate(results):
        assert result.result_code == ResultCode.OK
        assert result.record.bins["result"] == (i + 1) * 10 + 5


async def test_batch_operate_exp_write(exp_client_and_keys):
    """Test batch_operate with ExpOperation.write across multiple keys."""
    client, keys = exp_client_and_keys
    rp = ReadPolicy()

    expr = FilterExpression.num_mul([FilterExpression.int_bin("A"), FilterExpression.int_val(2)])
    ops = [ExpOperation.write("doubled", expr)]

    results = await client.batch_operate(keys, [ops] * len(keys), batch_policy=None, write_policy=None)

    assert len(results) == len(keys)
    for result in results:
        assert result.result_code == ResultCode.OK

    for i, key in enumerate(keys):
        rec = await client.get(key, policy=rp)
        assert rec.bins["doubled"] == (i + 1) * 10 * 2


async def test_batch_operate_exp_read_and_write(exp_client_and_keys):
    """Test batch_operate with combined ExpOperation.read + ExpOperation.write."""
    client, keys = exp_client_and_keys

    sum_expr = FilterExpression.num_add([FilterExpression.int_bin("A"), FilterExpression.int_bin("B")])
    read_expr = FilterExpression.int_bin("A")
    ops = [
        ExpOperation.write("sum", sum_expr),
        ExpOperation.read("a_val", read_expr),
    ]

    results = await client.batch_operate(keys, [ops] * len(keys), batch_policy=None, write_policy=None)

    assert len(results) == len(keys)
    for i, result in enumerate(results):
        assert result.result_code == ResultCode.OK
        assert result.record.bins["a_val"] == (i + 1) * 10


async def test_batch_operate_exp_with_regular_ops(exp_client_and_keys):
    """Test batch_operate mixing ExpOperation with regular Operation."""
    client, keys = exp_client_and_keys

    expr = FilterExpression.num_add([FilterExpression.int_bin("A"), FilterExpression.int_val(1)])
    ops = [
        Operation.put("tag", "processed"),
        ExpOperation.write("incremented", expr),
    ]

    results = await client.batch_operate(keys, [ops] * len(keys), batch_policy=None, write_policy=None)

    assert len(results) == len(keys)
    for result in results:
        assert result.result_code == ResultCode.OK

    rp = ReadPolicy()
    for i, key in enumerate(keys):
        rec = await client.get(key, policy=rp)
        assert rec.bins["tag"] == "processed"
        assert rec.bins["incremented"] == (i + 1) * 10 + 1


async def test_batch_operate_exp_read_eval_no_fail(exp_client_and_keys):
    """Test batch_operate with EVAL_NO_FAIL flag on missing bin."""
    client, keys = exp_client_and_keys

    expr = FilterExpression.int_bin("nonexistent")
    ops = [ExpOperation.read("result", expr, ExpReadFlags.EVAL_NO_FAIL)]

    results = await client.batch_operate(keys, [ops] * len(keys), batch_policy=None, write_policy=None)

    assert len(results) == len(keys)
    for result in results:
        assert result.result_code == ResultCode.OK
        assert result.record.bins.get("result") is None


async def test_batch_mixed_read_write_delete(client_and_keys):
    """Test Client.batch() with a mix of read, write, and delete ops in one call."""
    client, keys, delete_keys, bin_name = client_and_keys

    k_read = keys[0]
    k_write = Key("test", "test", "batch_mixed_write")
    k_delete = delete_keys[0]

    wp = WritePolicy()
    await client.put(k_write, {bin_name: "old_value"}, policy=wp)

    read_op = BatchReadOp(k_read, bins=[bin_name])
    write_op = BatchWriteOp(k_write, [Operation.put(bin_name, "new_value")])
    delete_op = BatchDeleteOp(k_delete)

    results = await client.batch([read_op, write_op, delete_op], batch_policy=None)

    assert len(results) == 3

    assert results[0].result_code == ResultCode.OK
    assert results[0].record.bins[bin_name] == "batchvalue1"

    assert results[1].result_code == ResultCode.OK

    assert results[2].result_code == ResultCode.OK

    rp = ReadPolicy()
    rec = await client.get(k_write, policy=rp)
    assert rec.bins[bin_name] == "new_value"

    exists = await client.exists(k_delete, policy=rp)
    assert exists is False


async def test_batch_mixed_with_invalid_namespace(client_and_keys):
    """A mixed batch with one invalid-namespace key records a per-key
    INVALID_NAMESPACE rather than failing the whole batch."""
    client, keys, _, bin_name = client_and_keys

    k_good = keys[0]
    k_bad = Key("invalid", "test", "k1")

    good_op = BatchWriteOp(k_good, [Operation.put(bin_name, "updated")])
    bad_op = BatchWriteOp(k_bad, [Operation.put(bin_name, "should_fail")])

    results = await client.batch([good_op, bad_op], batch_policy=None)

    assert len(results) == 2
    assert results[0].result_code == ResultCode.OK
    assert results[1].result_code == ResultCode.INVALID_NAMESPACE


async def test_batch_mixed_invalid_namespace_per_key(client_and_keys):
    """Mixed batch with one invalid-namespace key records per-key
    INVALID_NAMESPACE."""
    client, keys, delete_keys, bin_name = client_and_keys

    k_good = keys[0]
    k_bad = Key("invalid", "test", "k1")
    k_good2 = keys[5]
    k_del = delete_keys[0]

    results = await client.batch(
        [
        BatchWriteOp(k_good, [Operation.put("bin2", 100)]),
        BatchWriteOp(k_bad, [Operation.put("bin2", 100)]),
        BatchWriteOp(k_good2, [Operation.put("bin3", 999)]),
        BatchDeleteOp(k_del),
    ],
        batch_policy=None,
    )

    assert len(results) == 4
    assert results[0].result_code == ResultCode.OK
    assert results[1].result_code == ResultCode.INVALID_NAMESPACE
    assert results[2].result_code == ResultCode.OK
    assert results[3].result_code == ResultCode.OK


async def test_batch_mixed_with_policy(client_and_keys):
    """Test Client.batch() with per-op policies."""
    client, keys, _, bin_name = client_and_keys

    k = keys[1]

    bwp = BatchWritePolicy()
    bwp.record_exists_action = RecordExistsAction.CREATE_ONLY

    op = BatchWriteOp(k, [Operation.put("new_bin", 42)], policy=bwp)

    results = await client.batch([op], batch_policy=None)
    assert len(results) == 1
    assert results[0].result_code == ResultCode.KEY_EXISTS_ERROR


async def test_batch_single_key_fast_path_regression(client_and_keys):
    """Regression guard for the single-key fast path in core's batch executor.

    When a batch group contains exactly one key destined for a node, core
    routes that key through a single-key command (in
    ``aerospike-core/src/batch/batch_executor.rs::execute_single_op``) rather
    than the multi-key batch wire protocol. The previous CLIENT-2403
    regression in that path broke the batch contract in two ways:

    1. Per-key errors other than ``KEY_NOT_FOUND_ERROR`` /  ``FILTERED_OUT``
       (e.g. ``PARAMETER_ERROR``, ``KEY_EXISTS_ERROR``) propagated as a
       per-CALL exception instead of landing on the ``BatchRecord``.
    2. Delete-of-nonexistent returned per-record ``OK`` instead of
       ``KEY_NOT_FOUND_ERROR``.

    Both were fixed in aerospike-core PR #201 (merged into ``v3``).
    Individual coverage exists in :func:`test_batch_delete_key_not_found`,
    :func:`test_batch_mixed_with_policy`, and
    :func:`test_batch_read_invalid_filter_expression_bytes_returns_parameter_error`;
    this test consolidates all three single-key shapes in one place so a
    future refactor of the single-op fast path can be easily checked
    against the contract.
    """
    client, keys, _, bin_name = client_and_keys

    # (1) Delete-of-nonexistent must report KEY_NOT_FOUND_ERROR per-record.
    k_missing = Key("test", "test", "single_key_fast_path_missing")
    try:
        await client.delete(k_missing, policy=WritePolicy())
    except Exception:
        pass
    results = await client.batch_delete([k_missing], batch_policy=None, delete_policy=None)
    assert len(results) == 1
    assert results[0].result_code == ResultCode.KEY_NOT_FOUND_ERROR

    # (2) Per-key KEY_EXISTS_ERROR (CREATE_ONLY on existing key) must land on
    # the BatchRecord — not propagate as a per-CALL exception.
    bwp = BatchWritePolicy()
    bwp.record_exists_action = RecordExistsAction.CREATE_ONLY
    results = await client.batch(
        [BatchWriteOp(keys[0], [Operation.put("regression_v", 1)], policy=bwp)],
        batch_policy=None,
    )
    assert len(results) == 1
    assert results[0].result_code == ResultCode.KEY_EXISTS_ERROR

    # (3) Per-key PARAMETER_ERROR (invalid filter expression bytes) must land
    # on the BatchRecord — not propagate as a per-CALL exception.
    garbage_b64 = base64.b64encode(b"\xff\x00not-a-valid-expression").decode("ascii")
    brp = BatchReadPolicy()
    brp.filter_expression = FilterExpression.from_base64(garbage_b64)
    results = await client.batch_read(
        [keys[0]], [bin_name], batch_policy=None, read_policy=brp,
    )
    assert len(results) == 1
    assert results[0].result_code == ResultCode.PARAMETER_ERROR


async def test_batch_write_map_put(client_and_keys):
    """Client.batch BatchWriteOp accepts map CDT operations."""
    client, keys, _, _bin_name = client_and_keys

    k = Key("test", "test", "batch_cdt_map_write")
    wp = WritePolicy()
    await client.put(k, {"_init": 0}, policy=wp)

    mp = MapPolicy(None, None)
    results = await client.batch(
        [
        BatchWriteOp(k, [MapOperation.put("mb", "x", 99, mp)]),
    ],
        batch_policy=None,
    )
    assert len(results) == 1
    assert results[0].result_code == ResultCode.OK

    rec = await client.get(k, policy=ReadPolicy())
    assert rec.bins["mb"]["x"] == 99


async def test_batch_read_map_get_by_key(client_and_keys):
    """Client.batch BatchReadOp with map read operations."""
    client, keys, _, _bin_name = client_and_keys

    k = Key("test", "test", "batch_cdt_map_read")
    wp = WritePolicy()
    mp = MapPolicy(None, None)
    await client.put(k, {"_init": 0}, policy=wp)
    await client.operate(k, [MapOperation.put("mb", "k1", 42, mp)], policy=wp)

    results = await client.batch(
        [
        BatchReadOp(k, operations=[
            MapOperation.get_by_key("mb", "k1", MapReturnType.VALUE),
        ]),
    ],
        batch_policy=None,
    )
    assert len(results) == 1
    assert results[0].result_code == ResultCode.OK
    assert results[0].record.bins["mb"] == 42


async def test_batch_write_list_append(client_and_keys):
    """Client.batch BatchWriteOp accepts list CDT operations."""
    client, keys, _, _bin_name = client_and_keys

    k = keys[0]
    lp = ListPolicy(None, None)
    results = await client.batch(
        [
        BatchWriteOp(k, [ListOperation.append("lbin2", 7, lp)]),
    ],
        batch_policy=None,
    )
    assert len(results) == 1
    assert results[0].result_code == ResultCode.OK

    rec = await client.get(k, policy=ReadPolicy())
    assert rec.bins["lbin2"] == [0, 1, 7]


async def test_batch_write_with_nested_context(client_and_keys):
    """Client.batch applies CDT context on map operations (nested map key)."""
    client, _keys, _, _bin_name = client_and_keys

    k = Key("test", "test", "batch_cdt_nested_ctx")
    wp = WritePolicy()
    await client.delete(k, policy=wp)

    m1 = {"key11": 9, "key12": 4}
    m2 = {"key21": 3, "key22": 5}
    await client.put(k, {"mapbin": {"key1": m1, "key2": m2}}, policy=wp)

    mp = MapPolicy(None, None)
    results = await client.batch(
        [
        BatchWriteOp(k, [
            MapOperation.put("mapbin", "key21", 11, mp).set_context([CTX.map_key("key2")]),
        ]),
    ],
        batch_policy=None,
    )
    assert len(results) == 1
    assert results[0].result_code == ResultCode.OK

    rec = await client.get(k, ["mapbin"], policy=ReadPolicy())
    full_map = rec.bins["mapbin"]
    assert full_map["key2"]["key21"] == 11
    assert full_map["key2"]["key22"] == 5


async def test_batch_mixed_cdt_and_scalar(client_and_keys):
    """Client.batch BatchWriteOp can mix scalar Operation and MapOperation."""
    client, _keys, _, _bin_name = client_and_keys

    k = Key("test", "test", "batch_mixed_scalar_cdt")
    wp = WritePolicy()
    await client.put(k, {"_init": 0}, policy=wp)

    mp = MapPolicy(None, None)
    results = await client.batch(
        [
        BatchWriteOp(k, [
            Operation.put("txt", "hello"),
            MapOperation.put("mb", "a", 1, mp),
        ]),
    ],
        batch_policy=None,
    )
    assert len(results) == 1
    assert results[0].result_code == ResultCode.OK

    rec = await client.get(k, policy=ReadPolicy())
    assert rec.bins["txt"] == "hello"
    assert rec.bins["mb"]["a"] == 1


async def test_batch_write_bins_dict(client_and_keys):
    """BatchWriteOp ``bins`` writes each entry as a put."""
    client, _keys, _, _bin_name = client_and_keys

    k = Key("test", "test", "batch_write_bins_dict")
    await client.delete(k, policy=WritePolicy())

    results = await client.batch([BatchWriteOp(k, bins={"name": "Tim", "age": 31})])
    assert results[0].result_code == ResultCode.OK

    rec = await client.get(k, policy=ReadPolicy())
    assert rec.bins == {"name": "Tim", "age": 31}


async def test_batch_write_bins_precede_operations(client_and_keys):
    """BatchWriteOp applies ``bins`` before ``operations``, so a following add sees them."""
    client, _keys, _, _bin_name = client_and_keys

    k = Key("test", "test", "batch_write_bins_then_ops")
    await client.delete(k, policy=WritePolicy())

    results = await client.batch(
        [BatchWriteOp(k, [Operation.add("visits", 1)], bins={"visits": 10})],
    )
    assert results[0].result_code == ResultCode.OK

    rec = await client.get(k, policy=ReadPolicy())
    assert rec.bins["visits"] == 11


# ---------------------------------------------------------------------------
# Streaming batch (Client.batch_stream)
# ---------------------------------------------------------------------------

async def test_batch_stream_async_yields_all_records(client_and_keys):
    """`batch_stream` yields one BatchRecord per input op; set-equality on keys."""
    client, keys, _, bin_name = client_and_keys

    ops = [BatchReadOp(k, [bin_name]) for k in keys]
    stream = await client.batch_stream(ops)

    yielded = []
    async for idx, br in stream:
        yielded.append((idx, br))

    assert len(yielded) == len(keys)
    assert {idx for idx, _ in yielded} == set(range(len(keys)))
    # Every input key must appear exactly once across yielded records.
    yielded_key_values = {br.key.value for _, br in yielded}
    expected_key_values = {k.value for k in keys}
    assert yielded_key_values == expected_key_values
    for _, br in yielded:
        assert br.result_code == ResultCode.OK


async def test_batch_stream_async_per_key_error_inline(client_and_keys):
    """Per-key errors land on the BatchRecord — the stream does not raise."""
    client, keys, _, _bin_name = client_and_keys

    bwp = BatchWritePolicy()
    bwp.record_exists_action = RecordExistsAction.CREATE_ONLY

    # All target keys already exist (set up by fixture). CREATE_ONLY must
    # produce KEY_EXISTS_ERROR per-key, not raise.
    ops = [
        BatchWriteOp(keys[0], [Operation.put("new", 1)], policy=bwp),
        BatchWriteOp(keys[1], [Operation.put("new", 1)], policy=bwp),
        BatchReadOp(keys[2], None),
    ]
    stream = await client.batch_stream(ops)

    by_idx = {}
    async for idx, br in stream:
        by_idx[idx] = br

    assert len(by_idx) == 3
    assert by_idx[0].result_code == ResultCode.KEY_EXISTS_ERROR
    assert by_idx[1].result_code == ResultCode.KEY_EXISTS_ERROR
    assert by_idx[2].result_code == ResultCode.OK


async def test_batch_stream_async_mixed_ops(client_and_keys):
    """Mixed reads / writes / deletes in one stream — all yield without
    error, each with the right per-op result shape."""
    client, keys, delete_keys, bin_name = client_and_keys

    ops = [
        BatchReadOp(keys[0], [bin_name]),
        BatchWriteOp(keys[1], [Operation.put("stream_w", 42)]),
        BatchDeleteOp(delete_keys[0]),
        BatchReadOp(keys[2], None),
    ]
    stream = await client.batch_stream(ops)

    by_idx = {}
    async for idx, br in stream:
        by_idx[idx] = br

    assert set(by_idx.keys()) == {0, 1, 2, 3}
    for br in by_idx.values():
        assert br.result_code == ResultCode.OK


async def test_batch_stream_async_exhausted_stops(client_and_keys):
    """After the last record, `__anext__` raises StopAsyncIteration."""
    client, keys, _, bin_name = client_and_keys

    ops = [BatchReadOp(keys[0], [bin_name])]
    stream = await client.batch_stream(ops)

    count = 0
    async for _idx, _br in stream:
        count += 1
    assert count == 1

    # A second iteration over the same stream yields nothing — items already
    # consumed; the channel is closed.
    second_pass = []
    async for item in stream:
        second_pass.append(item)
    assert second_pass == []


# ---------------------------------------------------------------------------
# Batch-level filter expression
#
# Distinct from the per-record filters above: a filter set on BatchPolicy is
# written into the batch header, ahead of the batch-index field, so its
# presence changes that field's offset and declared size. The per-record
# BatchReadPolicy/BatchWritePolicy filters travel with each record instead and
# do not exercise that path.
# ---------------------------------------------------------------------------

def _match_filter(bin_name, value):
    return FilterExpression.eq(
        FilterExpression.int_bin(bin_name), FilterExpression.int_val(value)
    )


async def _seed_ints(client, keys, bin_name, values):
    """Reset each key to exactly one int bin.

    The shared fixture seeds list bins, and these tests add their own, so the
    records are deleted first -- otherwise a bin written by an earlier run makes
    "this write did not land" unprovable.
    """
    wp = WritePolicy()
    for key, value in zip(keys, values):
        try:
            await client.delete(key, policy=wp)
        except Exception:
            pass
        await client.put(key, {bin_name: value}, policy=wp)


async def test_batch_read_with_batch_level_filter_expression(client_and_keys):
    """A filter on BatchPolicy applies to every record in the batch."""
    client, keys, _, bin_name = client_and_keys
    pair = keys[:2]
    await _seed_ints(client, pair, bin_name, [1, 9])

    bp = BatchPolicy()
    bp.filter_expression = _match_filter(bin_name, 9)

    results = await client.batch_read(pair, [bin_name], batch_policy=bp)

    assert len(results) == len(pair)
    by_code = {r.key.value: r.result_code for r in results}
    assert by_code[pair[1].value] == ResultCode.OK
    assert by_code[pair[0].value] == ResultCode.FILTERED_OUT


async def test_batch_write_with_batch_level_filter_expression(client_and_keys):
    """A filter on BatchPolicy gates the write path, not just reads.

    This is the path that stamps the batch-index field after the filter.
    """
    client, keys, _, bin_name = client_and_keys
    pair = keys[2:4]
    await _seed_ints(client, pair, bin_name, [1, 9])

    bp = BatchPolicy()
    bp.filter_expression = _match_filter(bin_name, 9)

    results = await client.batch_write(
        pair, [{"tagged": 1}] * len(pair), batch_policy=bp
    )
    by_code = {r.key.value: r.result_code for r in results}
    assert by_code[pair[1].value] == ResultCode.OK
    assert by_code[pair[0].value] == ResultCode.FILTERED_OUT

    # The write reached the match and only the match.
    rp = ReadPolicy()
    matched = await client.get(pair[1], policy=rp)
    assert matched.bins.get("tagged") == 1
    skipped = await client.get(pair[0], policy=rp)
    assert "tagged" not in skipped.bins


async def test_batch_operate_with_batch_level_filter_expression(client_and_keys):
    """Same batch-level filter, through batch_operate rather than batch_write."""
    client, keys, _, bin_name = client_and_keys
    pair = keys[4:6]
    await _seed_ints(client, pair, bin_name, [1, 9])

    bp = BatchPolicy()
    bp.filter_expression = _match_filter(bin_name, 9)

    ops = [Operation.put("op_tagged", 1)]
    results = await client.batch_operate(pair, [ops] * len(pair), batch_policy=bp)

    by_code = {r.key.value: r.result_code for r in results}
    assert by_code[pair[1].value] == ResultCode.OK
    assert by_code[pair[0].value] == ResultCode.FILTERED_OUT


async def test_batch_level_filter_with_compression(client_and_keys):
    """Batch-level filter with request compression enabled.

    Compression shifts where the header's fields begin, so it combines with the
    filter to move the batch-index field twice over. Enterprise-only.
    """
    client, keys, _, bin_name = client_and_keys
    pair = keys[6:8]
    await _seed_ints(client, pair, bin_name, [1, 9])

    bp = BatchPolicy()
    bp.filter_expression = _match_filter(bin_name, 9)
    try:
        bp.use_compression = True
        bp.compression_threshold = 1  # compress even this small request
    except Exception as exc:
        pytest.skip(f"compression unavailable: {exc}")

    try:
        results = await client.batch_read(pair, [bin_name], batch_policy=bp)
    except ServerError as exc:
        if exc.result_code == ResultCode.ENTERPRISE_ONLY:
            pytest.skip("use_compression requires Aerospike Enterprise Edition")
        raise

    by_code = {r.key.value: r.result_code for r in results}
    assert by_code[pair[1].value] == ResultCode.OK
    assert by_code[pair[0].value] == ResultCode.FILTERED_OUT
