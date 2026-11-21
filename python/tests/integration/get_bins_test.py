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
import pytest_asyncio

from aerospike_native import (
    new_client,
    ClientPolicy,
    WritePolicy,
    ReadPolicy,
    Key,
    Blob,
    GeoJSON,
)


@pytest_asyncio.fixture
async def client_and_key():
    """Setup client and prepare test key."""
    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, os.environ.get("AEROSPIKE_HOST", "localhost:3000"))

    key = Key("test", "get_bins_test", "test_key")

    # Delete the record first
    wp = WritePolicy()
    rp = ReadPolicy()
    await client.delete(key, policy=wp)

    yield client, rp, key

    # Cleanup
    await client.delete(key, policy=wp)
    await client.close()


class TestGetBinsStandardTypes:
    """Test get_bins() with standard Python types."""

    async def test_get_bins_string(self, client_and_key):
        """Test get_bins() returns string as str."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": "hello world"}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        assert isinstance(bins["bin"], str)
        assert bins["bin"] == "hello world"

    async def test_get_bins_integer(self, client_and_key):
        """Test get_bins() returns integer as int."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": 42}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        assert isinstance(bins["bin"], int)
        assert bins["bin"] == 42

    async def test_get_bins_float(self, client_and_key):
        """Test get_bins() returns float as float."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": 3.14159}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        assert isinstance(bins["bin"], float)
        assert bins["bin"] == 3.14159

    async def test_get_bins_boolean(self, client_and_key):
        """Test get_bins() returns boolean as bool."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": True}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        assert isinstance(bins["bin"], bool)
        assert bins["bin"] is True

    async def test_get_bins_none(self, client_and_key):
        """Test get_bins() with None/nil bins (Aerospike doesn't return nil bins)."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        # First put a record with a non-Nil value (Aerospike requires at least one non-Nil bin)
        await client.put(key, {"placeholder": 1}, policy=wp)
        # Then put None to create a nil bin (Aerospike behavior: nil bins are not returned)
        await client.put(key, {"bin": None}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        # Aerospike doesn't return nil bins - they are omitted from the result
        assert "placeholder" in bins
        assert bins["placeholder"] == 1
        assert "bin" not in bins  # Nil bin is not returned


class TestGetBinsValueTypes:
    """Test get_bins() returns native Python types."""

    async def test_get_bins_list(self, client_and_key):
        """Test get_bins() returns a list bin as a Python list."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": [1, 2, 3]}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        # Returns Python native list
        assert isinstance(bins["bin"], list)
        assert bins["bin"] == [1, 2, 3]

    async def test_get_bins_list_nested(self, client_and_key):
        """Test get_bins() with nested lists."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": [1, [2, 3], 4]}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        # Returns Python native list
        assert isinstance(bins["bin"], list)
        assert bins["bin"][0] == 1
        assert bins["bin"][2] == 4
        assert isinstance(bins["bin"][1], list)
        assert bins["bin"][1] == [2, 3]

    async def test_get_bins_dict(self, client_and_key):
        """Test get_bins() returns a map bin as a Python dict."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        # Python dict input -> Python dict output
        await client.put(key, {"bin": {"a": 1, "b": 2}}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        assert isinstance(bins["bin"], dict)
        assert bins["bin"] == {"a": 1, "b": 2}

    async def test_get_bins_map_nested(self, client_and_key):
        """Test get_bins() with nested maps."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        # Python dict input -> Python dict output
        await client.put(key, {"bin": {"a": 1, "nested": {"b": 2}}}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        assert isinstance(bins["bin"], dict)
        assert bins["bin"]["a"] == 1
        assert isinstance(bins["bin"]["nested"], dict)
        assert bins["bin"]["nested"] == {"b": 2}

    async def test_get_bins_geojson_wrapper(self, client_and_key):
        """Test get_bins() returns GeoJSON wrapper class (current behavior)."""
        client, rp, key = client_and_key

        geo_str = '{"type": "Point", "coordinates": [-122.0, 37.5]}'
        wp = WritePolicy()
        await client.put(key, {"bin": GeoJSON(geo_str)}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        # Current behavior: returns GeoJSON wrapper class
        assert isinstance(bins["bin"], GeoJSON)
        assert bins["bin"].value == geo_str

    async def test_get_bins_blob_wrapper(self, client_and_key):
        """Test get_bins() returns Python native bytes (not wrapper class)."""
        client, rp, key = client_and_key

        blob_data = b"hello world"
        wp = WritePolicy()
        await client.put(key, {"bin": Blob(blob_data)}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        # Returns Python native bytes
        assert isinstance(bins["bin"], bytes)
        assert bins["bin"] == blob_data

    async def test_get_bins_hll_wrapper(self, client_and_key):
        """Test get_bins() returns HLL wrapper class."""
        # Note: HLL cannot be created via put() - it requires server-side HLL operations
        # However, if HLL values exist in the database (from server-side operations),
        # they will be returned as HLL wrapper objects (not native bytes)
        # This test documents the expected behavior but cannot easily create HLL values
        # to test with. In practice, HLL values retrieved from Aerospike will be
        # returned as HLL wrapper objects.
        pass


class TestGetBinsComplexNested:
    """Test get_bins() with complex nested structures."""

    async def test_get_bins_list_with_map(self, client_and_key):
        """Test get_bins() with list containing map."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": [1, {"a": 2}, 3]}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        # Returns Python native list
        assert isinstance(bins["bin"], list)
        assert bins["bin"][0] == 1
        assert bins["bin"][2] == 3
        assert isinstance(bins["bin"][1], dict)
        assert bins["bin"][1] == {"a": 2}

    async def test_get_bins_map_with_list(self, client_and_key):
        """Test get_bins() with map containing list."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": {"items": [1, 2, 3], "count": 3}}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        assert isinstance(bins["bin"], dict)
        assert bins["bin"]["count"] == 3
        assert isinstance(bins["bin"]["items"], list)
        assert bins["bin"]["items"] == [1, 2, 3]

    async def test_get_bins_deeply_nested(self, client_and_key):
        """Test get_bins() with deeply nested structures."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(
            key,
            {
                "bin": {
                    "level1": {
                        "level2": [1, {"level3": "value"}],
                    }
                }
            },
            policy=wp,
        )

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        assert isinstance(bins["bin"], dict)
        level1 = bins["bin"]["level1"]
        assert isinstance(level1, dict)
        level2 = level1["level2"]
        assert isinstance(level2, list)
        level3 = level2[1]
        assert isinstance(level3, dict)
        assert level3 == {"level3": "value"}


class TestGetBinsMultipleBins:
    """Test get_bins() with multiple bins."""

    async def test_get_bins_mixed_types(self, client_and_key):
        """Test get_bins() with mixed bin types."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(
            key,
            {
                "str": "hello",
                "int": 42,
                "float": 3.14,
                "bool": True,
                "list": [1, 2, 3],
                "map": {"a": 1},
                "geo": GeoJSON('{"type": "Point", "coordinates": [-122.0, 37.5]}'),
                "blob": Blob(b"data"),
            },
            policy=wp,
        )

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        assert isinstance(bins["str"], str)
        assert isinstance(bins["int"], int)
        assert isinstance(bins["float"], float)
        assert isinstance(bins["bool"], bool)
        assert isinstance(bins["list"], list)
        assert isinstance(bins["map"], dict)  # Python dict input -> Python dict output
        assert isinstance(bins["geo"], GeoJSON)  # GeoJSON still returns wrapper (not changed)
        assert isinstance(bins["blob"], bytes)  # Returns Python native bytes

        assert bins["str"] == "hello"
        assert bins["int"] == 42
        assert bins["float"] == 3.14
        assert bins["bool"] is True
        assert bins["list"] == [1, 2, 3]  # Direct list access
        assert bins["map"] == {"a": 1}
        assert bins["geo"].value == '{"type": "Point", "coordinates": [-122.0, 37.5]}'
        assert bins["blob"] == b"data"  # Direct bytes access


class TestBinMethod:
    """Test bin() method (single bin retrieval)."""

    async def test_bin_string(self, client_and_key):
        """Test bin() returns string as str."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": "hello"}, policy=wp)

        rec = await client.get(key, policy=rp)
        value = rec.bin("bin")

        assert isinstance(value, str)
        assert value == "hello"

    async def test_bin_list(self, client_and_key):
        """Test bin() returns a list bin as a Python list."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": [1, 2, 3]}, policy=wp)

        rec = await client.get(key, policy=rp)
        value = rec.bin("bin")

        # Returns Python native list
        assert isinstance(value, list)
        assert value == [1, 2, 3]

    async def test_bin_nonexistent(self, client_and_key):
        """Test bin() returns None for nonexistent bin."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"bin": "value"}, policy=wp)

        rec = await client.get(key, policy=rp)
        value = rec.bin("nonexistent")

        assert value is None


class TestGetBinsReturnType:
    """Test the return type of get_bins() itself."""

    async def test_get_bins_returns_dict(self, client_and_key):
        """Test get_bins() returns a dict-like object."""
        client, rp, key = client_and_key

        wp = WritePolicy()
        await client.put(key, {"a": 1, "b": 2}, policy=wp)

        rec = await client.get(key, policy=rp)
        bins = rec.bins

        # Should be dict-like
        assert hasattr(bins, "__getitem__")
        assert hasattr(bins, "__contains__")
        assert "a" in bins
        assert "b" in bins
        assert bins["a"] == 1
        assert bins["b"] == 2

