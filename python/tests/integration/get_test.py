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

import os
import pytest
import pytest_asyncio

from aerospike_native import ClientPolicy, new_client, ReadPolicy, WritePolicy, Key, FilterExpression as fe


@pytest_asyncio.fixture
async def client_and_key():
    """Setup client and create test record."""

    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, os.environ.get("AEROSPIKE_HOST", "localhost:3000"))

    rp = ReadPolicy()

    # make a record
    key = Key("test", "python_test", 1)
    wp = WritePolicy()

    await client.delete(key, policy=wp)

    await client.put(
        key,
        {
            "brand": "Ford",
            "model": "Mustang",
            "year": 1964,
            "fa/ir": "بر آن مردم دیده روشنایی سلامی چو بوی خوش آشنایی",
        },
        policy=wp,
    )

    yield client, rp, key
    await client.close()

async def test_all_bins(client_and_key):
    """Test getting all bins from a record."""

    client, rp, key = client_and_key
    rec = await client.get(key, policy=rp)
    assert rec is not None
    assert rec.generation == 1
    # assert rec.ttl is not None

async def test_bins_agree_in_any_access_order(client_and_key):
    """``bin()``, ``bins`` and ``repr()`` agree whichever is read first."""
    client, rp, key = client_and_key

    before = await client.get(key, ["brand", "year"], policy=rp)
    assert before.bin("brand") == "Ford"
    assert before.bins == {"brand": "Ford", "year": 1964}
    assert before.bin("brand") is before.bins["brand"]
    assert before.bin("model") is None

    after = await client.get(key, ["brand", "year"], policy=rp)
    assert "'brand': 'Ford'" in repr(after)
    assert after.bins == {"brand": "Ford", "year": 1964}
    assert after.bin("year") == 1964

async def test_some_bins(client_and_key):
    """Test getting specific bins from a record."""

    client, rp, key = client_and_key
    rec = await client.get(key, ["brand", "year"], policy=rp)
    assert rec is not None
    assert rec.bins == {"brand": "Ford", "year": 1964}

async def test_matching_filter_exp(client_and_key):
    """Test get operation with matching filter expression."""

    client, rp, key = client_and_key

    rp = ReadPolicy()
    rp.filter_expression = fe.eq(fe.string_bin("brand"), fe.string_val("Ford"))
    rec = await client.get(key, ["brand", "year"], policy=rp)
    assert rec is not None
    assert rec.bins == {"brand": "Ford", "year": 1964}

async def test_non_matching_filter_exp(client_and_key):
    """Test get operation with non-matching filter expression."""

    client, rp, key = client_and_key

    rp = ReadPolicy()
    rp.filter_expression = fe.eq(fe.string_bin("brand"), fe.string_val("Peykan"))

    with pytest.raises(Exception):
        await client.get(key, ["brand", "year"], policy=rp)
