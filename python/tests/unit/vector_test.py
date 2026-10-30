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

import array
import ctypes
import struct
import sys

import pytest

import aerospike_native
from aerospike_native import (
    BinDataType,
    FilterExpression,
    Order,
    ResultCode,
    Statement,
    Vector,
    VectorDistanceMetric,
    VectorElementType,
)
from aerospike_native.exceptions import ValueError as AerospikeValueError

NON_NATIVE_FLOAT = ctypes.c_float.__ctype_be__ if sys.byteorder == "little" else ctypes.c_float.__ctype_le__


def wire_bytes(type_code, fmt, values, version=1):
    """A vector wire value: version, element type, LE dimensions, LE elements."""
    return bytes([version, type_code]) + struct.pack(f"<I{len(values)}{fmt}", len(values), *values)


class TestVectorConstruction:
    @pytest.mark.parametrize(
        "element_type, values",
        [
            (VectorElementType.INT32, [-(2**31), 0, 2**31 - 1]),
            (VectorElementType.FLOAT32, [0.5, -1.25, 3.0]),
            (VectorElementType.FLOAT64, [0.1, -2.5, 1e300]),
            (VectorElementType.FLOAT16, [0.5, -2.0, 65504.0]),
        ],
    )
    def test_each_element_type_keeps_its_values(self, element_type, values):
        v = Vector(values, element_type)
        assert v.element_type == element_type
        assert v.dimensions == len(v) == len(values)
        assert v.value == values

    def test_default_element_type_is_float32(self):
        v = Vector((1, 2.5))
        assert v.element_type == VectorElementType.FLOAT32
        assert v.value == [1.0, 2.5]

    def test_float16_rounds_to_nearest_half(self):
        assert Vector([0.1], VectorElementType.FLOAT16).value == [0.0999755859375]

    def test_copy_of_vector(self):
        v = Vector([1.0, 2.0], VectorElementType.FLOAT64)
        assert Vector(v) == v
        with pytest.raises(TypeError):
            Vector(v, VectorElementType.FLOAT32)

    @pytest.mark.parametrize(
        "values, element_type",
        [
            ([], VectorElementType.FLOAT32),
            ([], VectorElementType.INT32),
            ([1.0, float("nan")], VectorElementType.FLOAT32),
            ([float("inf")], VectorElementType.FLOAT64),
            ([float("-inf")], VectorElementType.FLOAT16),
        ],
    )
    def test_empty_and_non_finite_are_parameter_errors(self, values, element_type):
        with pytest.raises(AerospikeValueError) as exc_info:
            Vector(values, element_type)
        assert exc_info.value.result_code == ResultCode.PARAMETER_ERROR

    def test_float16_out_of_range_is_rejected(self):
        with pytest.raises(AerospikeValueError, match="out of float16 range"):
            Vector([1.0, 70000.0], VectorElementType.FLOAT16)

    def test_oversized_vector_is_rejected(self):
        with pytest.raises(AerospikeValueError, match="exceeds max"):
            Vector([0.0] * 32769, VectorElementType.FLOAT64)

    def test_int32_overflow_and_non_numbers_are_rejected(self):
        with pytest.raises(OverflowError):
            Vector([2**31], VectorElementType.INT32)
        with pytest.raises(TypeError):
            Vector([1.0, "x"])
        with pytest.raises(TypeError):
            Vector("abc")


class TestVectorBuffer:
    @pytest.mark.parametrize(
        "typecode, element_type",
        [("i", VectorElementType.INT32), ("f", VectorElementType.FLOAT32), ("d", VectorElementType.FLOAT64)],
    )
    def test_array_typecode_selects_element_type(self, typecode, element_type):
        v = Vector(array.array(typecode, [1, -2, 3]))
        assert v.element_type == element_type
        assert v.value == [1, -2, 3]

    def test_half_precision_buffer_is_float16(self):
        # Python 3.11's memoryview cannot cast to 'e', but it can pass one through.
        source = Vector([0.5, -2.0], VectorElementType.FLOAT16)
        assert Vector(memoryview(source)) == source

    def test_strided_and_reversed_buffers_read_their_elements(self):
        values = memoryview(array.array("d", range(6)))
        assert Vector(values[::2]).value == [0.0, 2.0, 4.0]
        assert Vector(values[::-1]).value == [5.0, 4.0, 3.0, 2.0, 1.0, 0.0]

    def test_buffer_without_strides_is_read(self):
        native = (ctypes.c_float * 2)(1.5, -2.5)
        assert Vector(native) == Vector([1.5, -2.5])

    def test_invalid_buffers_are_rejected(self):
        with pytest.raises(AerospikeValueError, match="1-D"):
            Vector(memoryview(array.array("f", [0.0] * 4)).cast("B").cast("f", [2, 2]))
        with pytest.raises(TypeError, match="native byte order"):
            Vector((NON_NATIVE_FLOAT * 2)(1.0, 2.0))
        with pytest.raises(TypeError, match="unsupported buffer format"):
            Vector(array.array("q", [1, 2]))
        with pytest.raises(TypeError, match="unsupported buffer format"):
            Vector(b"\x00\x00\x80\x3f")
        with pytest.raises(TypeError, match="does not match"):
            Vector(array.array("f", [1.0]), VectorElementType.FLOAT64)
        with pytest.raises(AerospikeValueError):
            Vector(array.array("f", [float("nan")]))

    @pytest.mark.parametrize(
        "element_type, fmt",
        [
            (VectorElementType.FLOAT16, "e"),
            (VectorElementType.INT32, "i"),
            (VectorElementType.FLOAT32, "f"),
            (VectorElementType.FLOAT64, "d"),
        ],
    )
    def test_exports_a_read_only_buffer(self, element_type, fmt):
        view = memoryview(Vector([1, -2], element_type))
        assert (view.format, view.ndim, view.shape, view.readonly) == (fmt, 1, (2,), True)
        assert view.tobytes() == struct.pack(f"2{fmt}", 1, -2)

    @pytest.mark.parametrize("dtype", ["float16", "int32", "float32", "float64"])
    def test_numpy_round_trips_without_copying(self, dtype):
        np = pytest.importorskip("numpy")
        source = np.array([1, -2, 3], dtype=dtype)
        v = Vector(source)
        out = np.asarray(v)
        assert out.dtype == source.dtype
        np.testing.assert_array_equal(out, source)
        assert np.asarray(v).ctypes.data == out.ctypes.data
        assert not out.flags.writeable


class TestVectorSequence:
    def test_indexing_matches_the_value_list(self):
        v = Vector([1, 2, 3, 4], VectorElementType.INT32)
        assert (v[0], v[-1]) == (1, 4)
        assert v[1:3] == [2, 3]
        assert v[::-2] == [4, 2]
        assert list(v) == v.value

    def test_out_of_range_index_raises_index_error(self):
        v = Vector([1.0])
        with pytest.raises(IndexError):
            v[1]
        with pytest.raises(IndexError):
            v[-2]

    def test_float16_elements_read_as_floats(self):
        v = Vector([0.5, -2.0], VectorElementType.FLOAT16)
        assert (v[0], list(v)) == (0.5, [0.5, -2.0])


class TestVectorEquality:
    def test_equal_vectors_hash_alike(self):
        a = Vector([0.5, 1.0])
        b = Vector([0.5, 1.0])
        assert a == b
        assert hash(a) == hash(b)
        assert {a: "found"}[b] == "found"

    def test_element_type_and_elements_decide_equality(self):
        assert Vector([1.0], VectorElementType.FLOAT32) != Vector([1.0], VectorElementType.FLOAT64)
        assert Vector([1, 2], VectorElementType.INT32) != Vector([1, 3], VectorElementType.INT32)

    @pytest.mark.parametrize(
        "element_type",
        [VectorElementType.FLOAT16, VectorElementType.FLOAT32, VectorElementType.FLOAT64],
    )
    def test_negative_zero_equals_zero(self, element_type):
        a = Vector([-0.0, 1.0], element_type)
        b = Vector([0.0, 1.0], element_type)
        assert a == b
        assert hash(a) == hash(b)

    def test_not_equal_to_a_list(self):
        assert Vector([1.0]) != [1.0]

    def test_repr_reads_as_a_constructor(self):
        assert repr(Vector([1, 2], VectorElementType.INT32)) == "Vector([1, 2], VectorElementType.INT32)"


class TestVectorWireBytes:
    @pytest.mark.parametrize(
        "type_code, fmt, element_type, values",
        [
            (0x01, "e", VectorElementType.FLOAT16, [1.0, -2.0]),
            (0x02, "i", VectorElementType.INT32, [7, -8]),
            (0x03, "f", VectorElementType.FLOAT32, [1.5, -2.5]),
            (0x04, "d", VectorElementType.FLOAT64, [0.25, 1e300]),
        ],
    )
    def test_decodes_each_element_type(self, type_code, fmt, element_type, values):
        assert Vector.from_wire_bytes(wire_bytes(type_code, fmt, values)) == Vector(values, element_type)

    @pytest.mark.parametrize(
        "data",
        [
            wire_bytes(0x03, "f", [1.0]) + b"\x00",
            wire_bytes(0x03, "f", [1.0])[:-1],
            wire_bytes(0x03, "f", [1.0], version=2),
            wire_bytes(0x03, "f", []),
            wire_bytes(0x09, "f", [1.0]),
        ],
        ids=["trailing", "truncated", "unknown-version", "zero-dimensions", "unknown-type"],
    )
    def test_malformed_bytes_are_rejected(self, data):
        with pytest.raises(AerospikeValueError) as exc_info:
            Vector.from_wire_bytes(data)
        assert exc_info.value.result_code == ResultCode.PARAMETER_ERROR
        assert str(exc_info.value).count("Invalid argument") == 1


class TestVectorExpressions:
    @pytest.mark.parametrize(
        "metric",
        [
            VectorDistanceMetric.EUCLIDEAN_SQUARED,
            VectorDistanceMetric.DOT_PRODUCT,
            VectorDistanceMetric.COSINE_SIMILARITY,
        ],
    )
    def test_distance_builds_for_each_metric(self, metric):
        exp = FilterExpression.vector_distance(metric, Vector([1.0]), FilterExpression.vector_bin("embedding"))
        assert isinstance(exp, FilterExpression)


class TestStatementTopK:
    def test_set_order_by_replaces_and_then_order_by_appends(self):
        stmt = Statement("test", "products", ["score", "price"])
        stmt.set_order_by("score", BinDataType.INTEGER, Order.DESC)
        stmt.then_order_by("price", BinDataType.DOUBLE, Order.ASC)
        first, second = stmt.order_by
        assert (first.bin_name, first.data_type, first.order) == ("score", BinDataType.INTEGER, Order.DESC)
        assert (second.bin_name, second.data_type, second.order) == ("price", BinDataType.DOUBLE, Order.ASC)

        stmt.set_order_by("rank", BinDataType.INTEGER, Order.ASC)
        assert [key.bin_name for key in stmt.order_by] == ["rank"]

    def test_order_keys_are_integer_or_double_with_no_flags(self):
        assert not hasattr(BinDataType, "STRING")
        assert not hasattr(BinDataType, "BYTES")
        assert not hasattr(aerospike_native, "OrderByFlags")
        with pytest.raises(TypeError):
            Statement("test", "products").set_order_by("score", BinDataType.INTEGER, Order.ASC, 1)

    def test_top_k_defaults_to_none_and_can_be_cleared(self):
        stmt = Statement("test", "products")
        assert stmt.top_k is None
        assert stmt.order_by == []
        stmt.top_k = 10
        assert stmt.top_k == 10
        stmt.top_k = None
        assert stmt.top_k is None

    def test_order_by_compares_and_prints_by_value(self):
        stmt = Statement("test", "products")
        stmt.set_order_by("score", BinDataType.DOUBLE, Order.ASC)
        other = Statement("test", "products")
        other.set_order_by("score", BinDataType.DOUBLE, Order.ASC)
        assert stmt.order_by == other.order_by
        assert hash(stmt.order_by[0]) == hash(other.order_by[0])
        assert repr(stmt.order_by[0]) == "OrderBy('score', BinDataType.DOUBLE, Order.ASC)"
