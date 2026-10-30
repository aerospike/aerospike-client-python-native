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

import pytest
import pytest_asyncio

from aerospike_native import new_client, ClientPolicy, WritePolicy, ReadPolicy, Key, Operation, Expiration


@pytest_asyncio.fixture
async def client_and_key(aerospike_host):
    """Setup client and prepare test key."""
    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, aerospike_host)

    # Create a test key
    key = Key("test", "test", "opkey")

    # Delete the record first to ensure clean state
    wp = WritePolicy()
    await client.delete(key, policy=wp)

    yield client, key
    await client.close()

async def test_operate_put_and_get(client_and_key):
    """Test operate with Put and Get operations."""
    client, key = client_and_key

    wp = WritePolicy()

    # Write initial record
    await client.put(
        key,
        {
        "optintbin": 7,
        "optstringbin": "string value"
    },
        policy=wp,
    )

    # Use operate to put a new value and get the record
    # This is similar to: client.operate(null, key, Operation.put(bin4), Operation.get())
    record = await client.operate(
        key,
        [
            Operation.put("optstringbin", "new string"),
            Operation.get()
        ],
        policy=wp,
    )

    # Verify the record was returned
    assert record is not None
    assert record.bins is not None

    # Verify the new string value was written
    assert record.bins.get("optstringbin") == "new string"

    # Verify the original int bin is still there
    assert record.bins.get("optintbin") == 7


async def test_operate_get_only(client_and_key):
    """Test operate with Get operation only."""
    client, key = client_and_key

    wp = WritePolicy()

    # Write initial record
    await client.put(
        key,
        {
        "bin1": "value1",
        "bin2": 42
    },
        policy=wp,
    )

    # Use operate to get the record
    record = await client.operate(
        key,
        [Operation.get()],
        policy=wp,
    )

    # Verify the record was returned
    assert record is not None
    assert record.bins is not None
    assert record.bins.get("bin1") == "value1"
    assert record.bins.get("bin2") == 42


async def test_operate_multiple_puts(client_and_key):
    """Test operate with multiple Put operations."""
    client, key = client_and_key

    wp = WritePolicy()
    rp = ReadPolicy()

    # Write initial record
    await client.put(
        key,
        {
        "bin1": "original1",
        "bin2": "original2"
    },
        policy=wp,
    )

    # Use operate to put multiple bins and get the record
    record = await client.operate(
        key,
        [
            Operation.put("bin1", "updated1"),
            Operation.put("bin2", "updated2"),
            Operation.get()
        ],
        policy=wp,
    )

    # Verify the record was returned
    assert record is not None
    assert record.bins is not None

    # Verify both bins were updated
    assert record.bins.get("bin1") == "updated1"
    assert record.bins.get("bin2") == "updated2"

    # Verify by reading the record again
    rec = await client.get(key, policy=rp)
    assert rec.bins.get("bin1") == "updated1"
    assert rec.bins.get("bin2") == "updated2"


async def test_scalar_multi_op_results_are_op_aligned(client_and_key):
    """Multi-op results on one bin keep a slot per op, write-only ops included.

    ``get`` / ``add`` / ``get`` on the same bin comes back as
    ``[1, None, 11]`` — three ops, three slots, with the write-only ``add``
    occupying a slot with no value. The bins view and the positional view
    agree: neither drops the write's slot.
    """
    client, _ = client_and_key
    key = Key("test", "test", "scalar_multiop_positional")
    await client.delete(key)
    await client.put(key, {"n": 1})

    record = await client.operate(
        key,
        [
            Operation.get_bin("n"),
            Operation.add("n", 10),
            Operation.get_bin("n"),
        ],
    )

    assert record.results == [1, None, 11]
    assert record.bins["n"] == [1, None, 11]


async def test_operation_result_reads_the_built_results_list(client_and_key):
    """``operation_result(i)`` agrees with ``results`` whichever is read first."""
    client, _ = client_and_key
    key = Key("test", "test", "operation_result_after_results")
    await client.delete(key)
    await client.put(key, {"n": 1})
    ops = [Operation.get_bin("n"), Operation.add("n", 10), Operation.get_bin("n")]

    before = await client.operate(key, ops)
    assert before.operation_result(2) == 11
    assert before.results == [1, None, 11]

    after = await client.operate(key, ops)
    assert after.results == [11, None, 21]
    assert after.operation_result(0) is after.results[0]
    assert after.operation_result(1) is None
    assert after.operation_result(3) is None


async def test_operate_add_and_put(client_and_key):
    """Test operate with Add and Put operations."""
    client, key = client_and_key

    wp = WritePolicy()

    # Write initial record
    await client.put(
        key,
        {
        "optintbin": 7,
        "optstringbin": "string value"
    },
        policy=wp,
    )

    # Add integer, write new string and read record
    record = await client.operate(
        key,
        [
            Operation.add("optintbin", 4),
            Operation.put("optstringbin", "new string"),
            Operation.get()
        ],
        policy=wp,
    )

    # Verify the record was returned
    assert record is not None
    assert record.bins is not None

    # Verify the integer was added (7 + 4 = 11)
    assert record.bins.get("optintbin") == 11

    # Verify the new string value was written
    assert record.bins.get("optstringbin") == "new string"


async def test_operate_add_and_get(client_and_key):
    """Test operate with Add and Get operations."""
    client, key = client_and_key

    wp = WritePolicy()
    rp = ReadPolicy()

    # Delete record first to ensure clean state
    await client.delete(key, policy=wp)

    # Perform some adds
    await client.add(key, {"addbin": 10}, policy=wp)
    await client.add(key, {"addbin": 5}, policy=wp)

    # Verify the result
    rec = await client.get(key, ["addbin"], policy=rp)
    assert rec.bins.get("addbin") == 15

    # Test add and get combined
    record = await client.operate(
        key,
        [
            Operation.add("addbin", 30),
            Operation.get_bin("addbin")
        ],
        policy=wp,
    )

    # Verify the result (15 + 30 = 45): the add's slot is None, the read's
    # slot carries the new value.
    assert record is not None
    assert record.results == [None, 45]
    assert record.bins.get("addbin") == [None, 45]


async def test_operate_append(client_and_key):
    """Test operate with Append operation."""
    client, key = client_and_key

    wp = WritePolicy()
    rp = ReadPolicy()

    # Delete record first to ensure clean state
    await client.delete(key, policy=wp)

    # Append operations
    await client.append(key, {"appendbin": "Hello"}, policy=wp)
    await client.append(key, {"appendbin": " World"}, policy=wp)

    # Verify the result
    rec = await client.get(key, ["appendbin"], policy=rp)
    assert rec.bins.get("appendbin") == "Hello World"

    # Test append in operate
    record = await client.operate(
        key,
        [
            Operation.append("appendbin", "!"),
            Operation.get_bin("appendbin")
        ],
        policy=wp,
    )

    # Verify the result: the append's slot is None, the read's slot carries
    # the appended string.
    assert record is not None
    assert record.results == [None, "Hello World!"]
    assert record.bins.get("appendbin") == [None, "Hello World!"]


async def test_operate_prepend(client_and_key):
    """Test operate with Prepend operation."""
    client, key = client_and_key

    wp = WritePolicy()
    rp = ReadPolicy()

    # Delete record first to ensure clean state
    await client.delete(key, policy=wp)

    # Prepend operations
    await client.prepend(key, {"prependbin": "World"}, policy=wp)
    await client.prepend(key, {"prependbin": "Hello "}, policy=wp)

    # Verify the result
    rec = await client.get(key, ["prependbin"], policy=rp)
    assert rec.bins.get("prependbin") == "Hello World"

    # Test prepend in operate
    record = await client.operate(
        key,
        [
            Operation.prepend("prependbin", "Say: "),
            Operation.get_bin("prependbin")
        ],
        policy=wp,
    )

    # Verify the result: the prepend's slot is None, the read's slot carries
    # the prepended string.
    assert record is not None
    assert record.results == [None, "Say: Hello World"]
    assert record.bins.get("prependbin") == [None, "Say: Hello World"]


async def test_operate_get_header(client_and_key):
    """Test operate with GetHeader operation."""
    client, key = client_and_key

    wp = WritePolicy()

    # Write initial record
    await client.put(
        key,
        {
        "mybin": "myvalue"
    },
        policy=wp,
    )

    # Use operate to get header (metadata only, no bin data)
    record = await client.operate(
        key,
        [Operation.get_header()],
        policy=wp,
    )

    # Verify the record was returned
    assert record is not None
    # GetHeader should return no bins (metadata only)
    assert record.bins is None or len(record.bins) == 0
    # Generation should be greater than zero
    assert record.generation is not None
    assert record.generation > 0


async def test_operate_delete(client_and_key):
    """Test operate with Delete operation."""
    client, key = client_and_key

    wp = WritePolicy()
    rp = ReadPolicy()

    # Write initial record
    await client.put(
        key,
        {
        "optintbin1": 1
    },
        policy=wp,
    )

    # Read bin1 and then delete all
    record = await client.operate(
        key,
        [
            Operation.get_bin("optintbin1"),
            Operation.delete()
        ],
        policy=wp,
    )

    # Verify we got the bin value before deletion
    assert record is not None
    assert record.bins is not None
    assert record.bins.get("optintbin1") == 1

    # Verify record is gone
    exists = await client.exists(key, policy=rp)
    assert not exists

    # Rewrite record with two bins
    await client.put(
        key,
        {
        "optintbin1": 1,
        "optintbin2": 2
    },
        policy=wp,
    )

    # Read bin 1 and then delete all followed by a write of bin2
    record = await client.operate(
        key,
        [
            Operation.get_bin("optintbin1"),
            Operation.delete(),
            Operation.put("optintbin2", 2),
            Operation.get_bin("optintbin2")
        ],
        policy=wp,
    )

    # Verify we got bin1 value before deletion
    assert record is not None
    assert record.bins is not None
    assert record.bins.get("optintbin1") == 1

    # Read record - should only have bin2
    rec = await client.get(key, policy=rp)
    assert rec is not None
    assert rec.bins.get("optintbin2") == 2
    # bin1 should be gone
    assert "optintbin1" not in rec.bins or rec.bins.get("optintbin1") is None
    # Should only have one bin
    assert len(rec.bins) == 1


async def test_operate_touch_and_get_header(client_and_key):
    """Test operate with Touch and GetHeader operations.

    If TTL is not available on the server, the test will be skipped.
    """
    client, key = client_and_key

    wp = WritePolicy()
    rp = ReadPolicy()

    # Check if TTL is supported by trying to set expiration
    try:
        wp.expiration = Expiration.seconds(60)
        await client.put(
            key,
            {
            "touchbin": "touchvalue"
        },
            policy=wp,
        )
    except Exception as e:
        # If setting expiration fails, TTL is not supported - skip the test
        pytest.skip(f"TTL not supported on this server: {e}")

    # Verify TTL was actually set on the record
    rp_check = ReadPolicy()
    check_rec = await client.get(key, ["touchbin"], policy=rp_check)
    if check_rec.ttl == 0:
        pytest.skip("TTL not being applied by server (got ttl=0)")

    wp.expiration = Expiration.seconds(120)
    record = await client.operate(
        key,
        [
            Operation.touch(),
            Operation.get_header()
        ],
        policy=wp,
    )

    assert record is not None
    # Neither op reads a bin, so no bin value comes back (the touch's slot
    # is None, under an empty bin name).
    assert all(value is None for value in (record.bins or {}).values())
    # TTL should be set (expiration > 0)
    assert record.ttl is not None
    assert record.ttl > 0, f"Expected TTL > 0 after touch with expiration=120s, got {record.ttl}"

    rec = await client.get(key, ["touchbin"], policy=rp)
    assert rec is not None
    assert rec.bins.get("touchbin") == "touchvalue"
    assert rec.ttl > 0, f"Record TTL should be > 0, got {rec.ttl}"


