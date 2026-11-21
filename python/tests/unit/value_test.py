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
from aerospike_native import GeoJSON, Blob, HLL, Key, geojson, null

# Common test data
TEST_BLOB_DATA_1 = [1, 7, 8, 4, 1]
TEST_BLOB_DATA_2 = [1, 7, 8, 4]
TEST_BLOB_DATA_3 = [1, 2, 3]

def test_geo_json_equality():
    """Test GeoJSON object creation and equality."""

    geo_str = '{"type":"Point","coordinates":[-80.590003, 28.60009]}'
    geo = GeoJSON('{"type":"Point","coordinates":[-80.590003, 28.60009]}')
    geo2 = GeoJSON('{"type":"Point","coordinates":[-80.590003, 28.60009]}')

    assert geo_str == geo == geo2

def test_geo_json_inequality():
    """Test GeoJSON object inequality."""

    geo_str = '{"type":"Point","coordinates":[-80.590003, 28.60009]}'
    geo_different_str = '{"type":"Point","coordinates":[-80.590003, 28.60008]}'
    
    geo = GeoJSON(geo_str)
    different_geo = GeoJSON(geo_different_str)
    
    assert geo_str != different_geo
    assert geo != different_geo

def test_geo_json_set_and_get():
    """Test GeoJSON value setting and getting."""

    geo_str = '{"type":"Point","coordinates":[-80.590003, 28.60009]}'
    geo_different_str = '{"type":"Point","coordinates":[-80.590003, 28.60008]}'
    
    geo = GeoJSON(geo_str)
    geo.value = geo_different_str
    assert geo.value == geo_different_str

def test_geo_json_str_repr():
    """Test GeoJSON string representation."""

    geo_str = '{"type":"Point","coordinates":[-80.590003, 28.60009]}'
    geo = GeoJSON(geo_str)
    
    assert str(geo) == geo_str
    assert repr(geo) == f"GeoJSON({geo_str})"


def test_geo_json_from_dict():
    """Test GeoJSON creation from dictionary."""
    geo_dict = {"type": "Point", "coordinates": [-80.590003, 28.60009]}
    geo = GeoJSON(geo_dict)

    # Python's json.dumps adds spaces, so we parse and compare the structure
    import json
    parsed = json.loads(geo.value)
    assert parsed == geo_dict
    assert geo.value.startswith('{"type":')
    assert '"coordinates"' in geo.value


def test_geo_json_from_dict_polygon():
    """Test GeoJSON creation from dictionary with Polygon type."""
    geo_dict = {
        "type": "Polygon",
        "coordinates": [[[-122.0, 37.0], [-121.0, 37.0], [-121.0, 38.0], [-122.0, 38.0], [-122.0, 37.0]]]
    }
    geo = GeoJSON(geo_dict)

    assert "type" in geo.value
    assert "Polygon" in geo.value
    assert "coordinates" in geo.value


def test_geo_json_from_dict_equality():
    """Test that GeoJSON created from dict equals GeoJSON created from string."""
    geo_dict = {"type": "Point", "coordinates": [-80.590003, 28.60009]}
    geo_from_dict = GeoJSON(geo_dict)

    geo_str = '{"type":"Point","coordinates":[-80.590003,28.60009]}'
    geo_from_str = GeoJSON(geo_str)

    # Note: JSON serialization may add/remove spaces, so compare parsed values
    import json
    assert json.loads(geo_from_dict.value) == json.loads(geo_from_str.value)

def test_blob_set_and_get():
    """Test Blob value setting and getting."""

    blob = Blob(TEST_BLOB_DATA_1)
    blob.value = [2, 3, 4]
    assert blob.value == bytes([2, 3, 4])

def test_blob_equality():
    """Test Blob object equality."""

    blob = Blob(TEST_BLOB_DATA_1)
    blob2 = bytearray(TEST_BLOB_DATA_1)
    blob3 = bytes(TEST_BLOB_DATA_1)
    blob4 = Blob(blob2)
    blob5 = Blob(blob3)

    assert blob == blob2
    assert blob == blob3
    assert blob == blob4
    assert blob == blob5
    assert blob4 == blob5

def test_blob_inequality():
    """Test Blob object inequality."""

    blob = Blob(TEST_BLOB_DATA_1)
    blob2 = bytearray(TEST_BLOB_DATA_2)
    blob3 = bytes(TEST_BLOB_DATA_2)
    blob4 = Blob(blob3)

    assert blob != blob4
    assert blob != blob2
    assert blob != blob3

def test_blob_get_by_index():
    """Test Blob indexing."""

    blob = Blob(TEST_BLOB_DATA_1)
    assert blob[0] == 1

def test_blob_get_by_index_fail():
    """Test Blob indexing out of bounds raises IndexError."""

    blob = Blob(TEST_BLOB_DATA_1)
    with pytest.raises(IndexError) as exc_info:
        blob[5]
    assert exc_info.value.args[0] == "index out of bounds"

def test_blob_set_by_index():
    """Test Blob assignment by index."""

    blob = Blob(TEST_BLOB_DATA_1)
    blob[0] = 1

def test_blob_set_by_index_fail():
    """Test Blob assignment out of bounds raises IndexError."""

    blob = Blob(TEST_BLOB_DATA_1)
    with pytest.raises(IndexError) as exc_info:
        blob[5] = 0
    assert exc_info.value.args[0] == "index out of bounds"

def test_blob_delete():
    """Test Blob item deletion."""

    blob = Blob(TEST_BLOB_DATA_3)
    del blob[0]
    assert blob == Blob(bytes([2, 3]))

def test_blob_concat():
    """Test Blob concatenation."""

    blob1 = Blob(bytes([1]))
    blob2 = Blob(bytes([2]))
    assert Blob(bytes([1, 2])) == blob1 + blob2

def test_blob_concat_fail():
    """Test failed Blob + String concatenation."""

    blob = Blob(bytes([1]))
    string = "bad_news"
    with pytest.raises(TypeError) as exc_info:
        blob + string
    assert exc_info.value.args[0] == "unsupported operand type(s) for +: 'Blob' and other type"

def test_blob_repeat():
    """Test Blob repetition."""

    blob = Blob(bytes([1]))
    assert blob * 3 == Blob(bytes([1, 1, 1]))

def test_blob_inplace_concat():
    """Test Blob in-place concatenation."""

    blob = Blob(bytes([1]))
    blob += Blob(bytes([2, 3]))
    assert blob == Blob(bytes([1, 2, 3]))

def test_blob_inplace_repeat():
    """Test Blob in-place repetition."""

    blob = Blob(bytes([1]))
    blob *= 3
    assert blob == Blob(bytes([1, 1, 1]))

def test_blob_hash():
    """Test Blob hashing for dictionary keys."""

    blob_bytes = bytes(TEST_BLOB_DATA_1)
    blob = Blob(blob_bytes)
    d = {1: blob, blob: 1}
    d2 = {1: blob_bytes, blob: 1}
    assert d == d2

def test_hll_equality():
    """Test HLL object equality."""

    hll = HLL(bytes([1, 2, 3, 4]))
    b = bytes([1, 2, 3, 4])
    hll2 = HLL(bytes([1, 2, 3, 4]))

    assert hll == b
    assert hll == hll2

def test_hll_inequality():
    """Test HLL object inequality."""

    hll = HLL(bytes([1, 2, 3, 4]))
    b = bytes([1, 2, 3, 5])
    hll2 = HLL(bytes([1, 2, 3, 5]))

    assert hll != b
    assert hll != hll2

def test_hll_set_and_get():
    """Test HLL value setting and getting."""

    hll = HLL(bytes([1, 2, 3, 4]))
    hll.value = [5, 6, 7]
    assert hll.value == bytes([5, 6, 7])


def test_geojson_helper_function():
    """Test geojson() helper function that converts coordinate strings to GeoJSON."""
    geo = geojson("-122.0, 37.5")

    assert isinstance(geo, GeoJSON)
    assert "type" in geo.value
    assert "Point" in geo.value
    assert "coordinates" in geo.value

    # Check coordinates are correct
    import json
    geo_data = json.loads(geo.value)
    assert geo_data["type"] == "Point"
    assert geo_data["coordinates"] == [-122.0, 37.5]


def test_geojson_helper_function_negative_coords():
    """Test geojson() helper with negative coordinates."""
    geo = geojson("-80.590003, 28.60009")

    import json
    geo_data = json.loads(geo.value)
    assert geo_data["coordinates"] == [-80.590003, 28.60009]


def test_geojson_helper_function_json_string():
    """Test geojson() normalizes a JSON string the same way the dict constructor does."""
    geo_dict = {"type": "Point", "coordinates": [-122.0, 37.5]}
    # Repeated calls exercise the cached json callables, not just their first resolution.
    for _ in range(2):
        geo = geojson('{"type":"Point",  "coordinates":[-122.0,37.5]}')
        assert geo == GeoJSON(geo_dict)


def test_geojson_helper_function_invalid():
    """Test geojson() helper with invalid coordinate string."""
    with pytest.raises(ValueError):
        geojson("invalid")

    with pytest.raises(ValueError):
        geojson("122.0")  # Missing comma

    with pytest.raises(ValueError):
        geojson("122.0, 37.5, 10.0")  # Too many coordinates


def test_null_function():
    """Test null() helper function returns None."""
    null_val = null()
    assert null_val is None


def test_u64_large_integer():
    """Test that large positive integers (u64) overflow to negative i64.

    Note: Since Value::UInt was removed from the Rust core, values > i64::MAX
    will overflow when converted to i64. This test verifies the overflow behavior.
    A key's user value goes through the same conversion as a bin value and
    reads back without a server.
    """
    # i64::MAX is 9223372036854775807
    # Test with a value larger than i64::MAX - it will overflow to negative
    large_uint = 2**63 + 1000  # 9223372036854775808 + 1000
    # This overflows: 9223372036854776808 -> -9223372036854774808 (i64::MIN + (value - i64::MAX - 1))
    expected_overflow = large_uint - 2**64  # Overflow calculation

    assert Key("test", "values", large_uint).value == expected_overflow


def test_u64_boundary_values():
    """Test u64 boundary values (i64::MAX and i64::MAX + 1).

    Note: Since Value::UInt was removed from the Rust core, i64::MAX + 1
    will overflow to i64::MIN when converted to i64.
    """
    i64_max = 2**63 - 1  # 9223372036854775807
    i64_max_plus_one = 2**63  # 9223372036854775808

    # i64_max works fine, but i64_max_plus_one overflows to i64::MIN
    assert Key("test", "values", i64_max).value == i64_max
    assert Key("test", "values", i64_max_plus_one).value == -9223372036854775808  # i64::MIN (overflow)


def test_special_value_enum_distinct():
    """CDT boundary markers are distinct enum members."""
    from aerospike_native import SpecialValue

    assert SpecialValue.NULL != SpecialValue.INFINITY
    assert SpecialValue.INFINITY != SpecialValue.WILDCARD
    assert "SpecialValue.INFINITY" == repr(SpecialValue.INFINITY)


def test_map_operation_accepts_special_value_range_endpoints():
    """Map get_by_key_range accepts SpecialValue for open-ended CDT ranges."""
    from aerospike_native import MapOperation, MapReturnType, SpecialValue

    op = MapOperation.get_by_key_range(
        "m", SpecialValue.NULL, SpecialValue.INFINITY, MapReturnType.KEY,
    )
    assert op is not None


def test_list_operation_accepts_special_value_range_endpoints():
    from aerospike_native import ListOperation, ListReturnType, SpecialValue

    op = ListOperation.get_by_value_range(
        "lst", 1, SpecialValue.INFINITY, ListReturnType.VALUE,
    )
    assert op is not None
