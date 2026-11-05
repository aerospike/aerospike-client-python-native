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

"""Vector bins, vector-distance expressions, and Top-K queries."""

import array
import os

import pytest
from aerospike_native import (
    BinDataType,
    ExpOperation,
    ExpReadFlags,
    FilterExpression,
    Key,
    ListOperation,
    ListPolicy,
    Operation,
    Order,
    PartitionFilter,
    QueryPolicy,
    ResultCode,
    Statement,
    UDFLang,
    Vector,
    VectorDistanceMetric,
    VectorElementType,
)
from aerospike_native.exceptions import InvalidRequest, ServerError
from aerospike_native.exceptions import ValueError as AerospikeValueError
from fixtures import TestFixtureCleanDB

NAMESPACE = "test"
SET = "test"


def distance(metric, query, bin_name="embedding"):
    return FilterExpression.vector_distance(metric, query, FilterExpression.vector_bin(bin_name))


async def collect(client, stmt, policy=None):
    records = await client.query(stmt, PartitionFilter.all(), policy=policy or QueryPolicy())
    return [record async for record in records]


class TestVectorBins(TestFixtureCleanDB):
    @pytest.fixture(autouse=True)
    def require_vector(self, supports_vector):
        if not supports_vector:
            pytest.skip("cluster lacks vector support (Node.version.supports_vector() is False)")

    @pytest.mark.parametrize(
        "vector",
        [
            Vector([1, -2, 2**31 - 1], VectorElementType.INT32),
            Vector([0.5, -1.25, 3.0], VectorElementType.FLOAT32),
            Vector([0.1, -2.5, 1e300], VectorElementType.FLOAT64),
            Vector([0.5, -2.0, 65504.0], VectorElementType.FLOAT16),
        ],
        ids=["int32", "float32", "float64", "float16"],
    )
    async def test_put_get_round_trip(self, client, key, vector):
        await client.put(key, {"embedding": vector})
        record = await client.get(key)
        assert record.bins["embedding"] == vector
        assert record.bins["embedding"].element_type == vector.element_type

    async def test_buffer_round_trip(self, client, key):
        embedding = array.array("f", (i / 768 for i in range(768)))
        await client.put(key, {"embedding": Vector(embedding)})
        record = await client.get(key)
        assert memoryview(record.bins["embedding"]) == memoryview(embedding)

    async def test_overwrite_with_different_dimensions(self, client, key):
        await client.put(key, {"embedding": Vector([1.0, 2.0, 3.0])})
        replacement = Vector([4.0, 5.0], VectorElementType.FLOAT64)
        await client.put(key, {"embedding": replacement})
        assert (await client.get(key)).bins["embedding"] == replacement

    async def test_operate_put_and_get(self, client, key):
        vector = Vector([7, 8, 9], VectorElementType.INT32)
        await client.operate(key, [Operation.put("embedding", vector)])
        record = await client.operate(key, [Operation.get_bin("embedding")])
        assert record.bins["embedding"] == vector

    async def test_batch_operate_reads_each_vector(self, client):
        keys = [Key(NAMESPACE, SET, i) for i in range(3)]
        vectors = [Vector([float(i), float(i + 1)]) for i in range(3)]
        writes = [[Operation.put("embedding", v)] for v in vectors]
        for result in await client.batch_operate(keys, writes, batch_policy=None, write_policy=None):
            assert result.result_code == ResultCode.OK

        reads = await client.batch_operate(
            keys, [[Operation.get_bin("embedding")]] * len(keys), batch_policy=None, write_policy=None,
        )
        assert [result.record.bins["embedding"] for result in reads] == vectors

    @pytest.mark.parametrize(
        "bins",
        [{"items": [Vector([1.0])]}, {"lookup": {"k": Vector([1.0])}}],
        ids=["list", "map"],
    )
    async def test_vector_nested_in_collection_is_rejected_by_server(self, client, key, bins):
        with pytest.raises(InvalidRequest) as exc_info:
            await client.put(key, bins)
        assert exc_info.value.result_code == ResultCode.PARAMETER_ERROR

    async def test_vector_as_list_append_value_is_rejected_by_server(self, client, key):
        with pytest.raises(InvalidRequest) as exc_info:
            await client.operate(key, [ListOperation.append("items", Vector([1.0]), ListPolicy())])
        assert exc_info.value.result_code == ResultCode.PARAMETER_ERROR

    @pytest.fixture
    async def vector_udf(self, client):
        udf_path = os.path.join(os.path.dirname(__file__), "udf", "vector_example.lua")
        task = await client.register_udf_from_file(udf_path, "vector_example.lua", UDFLang.LUA)
        assert await task.wait_till_complete()
        yield "vector_example"
        remove_task = await client.remove_udf("vector_example.lua")
        await remove_task.wait_till_complete()

    async def test_udf_returns_vector(self, client, key, vector_udf):
        vector = Vector([1.5, -2.0], VectorElementType.FLOAT64)
        await client.put(key, {"embedding": vector})
        assert await client.execute_udf(key, vector_udf, "echo", [vector]) == vector
        assert await client.execute_udf(key, vector_udf, "read_bin", ["embedding"]) == vector

    async def test_aggregation_streams_vectors(self, client, vector_udf):
        vectors = [Vector([float(i), -float(i)], VectorElementType.FLOAT64) for i in range(3)]
        for i, vector in enumerate(vectors):
            await client.put(Key(NAMESPACE, SET, i), {"embedding": vector})
        stmt = Statement(NAMESPACE, SET)
        stmt.set_aggregate_function(vector_udf, "stream_bin", ["embedding"])
        streamed = [record.bins["SUCCESS"] for record in await collect(client, stmt)]
        assert sorted(streamed, key=lambda v: v[0]) == vectors


class TestVectorDistance(TestFixtureCleanDB):
    @pytest.fixture(autouse=True)
    def require_vector(self, supports_vector):
        if not supports_vector:
            pytest.skip("cluster lacks vector support (Node.version.supports_vector() is False)")

    @pytest.fixture
    async def stored(self, client, key):
        await client.put(key, {"embedding": Vector([2.0, 0.0, 1.0], VectorElementType.FLOAT64)})
        return key

    @pytest.mark.parametrize(
        "metric, expected",
        [
            (VectorDistanceMetric.EUCLIDEAN_SQUARED, 9.0),
            (VectorDistanceMetric.DOT_PRODUCT, 5.0),
            (VectorDistanceMetric.COSINE_SIMILARITY, 5.0 / 70**0.5),
        ],
    )
    async def test_metric_value(self, client, stored, metric, expected):
        query = Vector([1.0, 2.0, 3.0], VectorElementType.FLOAT64)
        record = await client.operate(stored, [ExpOperation.read("d", distance(metric, query), ExpReadFlags.DEFAULT)])
        assert record.bins["d"] == pytest.approx(expected)

    async def test_filter_by_distance(self, client, stored):
        query = Vector([1.0, 2.0, 3.0], VectorElementType.FLOAT64)
        near = FilterExpression.lt(
            distance(VectorDistanceMetric.EUCLIDEAN_SQUARED, query), FilterExpression.float_val(10.0)
        )
        assert (await client.get(stored, filter_expression=near)).bins["embedding"] is not None

        far = FilterExpression.lt(
            distance(VectorDistanceMetric.EUCLIDEAN_SQUARED, query), FilterExpression.float_val(1.0)
        )
        with pytest.raises(ServerError) as exc_info:
            await client.get(stored, filter_expression=far)
        assert exc_info.value.result_code == ResultCode.FILTERED_OUT

    async def test_mismatched_dimensions_evaluate_to_unknown(self, client, stored):
        mismatched = distance(VectorDistanceMetric.DOT_PRODUCT, Vector([1.0, 2.0], VectorElementType.FLOAT64))
        record = await client.operate(stored, [ExpOperation.read("d", mismatched, ExpReadFlags.EVAL_NO_FAIL)])
        assert record.bins.get("d") is None


class TestTopK(TestFixtureCleanDB):
    @pytest.fixture(autouse=True)
    def require_top_k(self, supports_query_top_k):
        if not supports_query_top_k:
            pytest.skip("cluster lacks Top-K (Node.version.supports_query_top_k() is False)")

    @pytest.fixture
    async def products(self, client):
        rows = [
            (1, 9.5, b"\x09", 40, [0.0, 0.0]),
            (2, 7.25, b"\x07\x01", 90, [3.0, 4.0]),
            (3, 7.0, b"\x07", 90, [1.0, 1.0]),
            (4, 5.0, b"\x05", 70, [0.5, 0.2]),
            (5, 3.0, b"\x03", None, [5.0, 5.0]),
        ]
        for pid, price, sku, score, embedding in rows:
            bins = {"id": pid, "price": price, "sku": sku,
                    "embedding": Vector(embedding, VectorElementType.FLOAT64)}
            if score is not None:
                bins["score"] = score
            await client.put(Key(NAMESPACE, SET, pid), bins)
        return rows

    async def test_highest_scores_first(self, client, products):
        stmt = Statement(NAMESPACE, SET, ["id", "score"])
        stmt.set_order_by("score", BinDataType.INTEGER, Order.DESC)
        stmt.top_k = 3
        records = await collect(client, stmt)
        assert [r.bins["score"] for r in records] == [90, 90, 70]

    async def test_second_key_breaks_ties(self, client, products):
        stmt = Statement(NAMESPACE, SET, ["id", "price", "score"])
        stmt.set_order_by("score", BinDataType.INTEGER, Order.DESC)
        stmt.then_order_by("price", BinDataType.DOUBLE, Order.ASC)
        stmt.top_k = 2
        assert [r.bins["id"] for r in await collect(client, stmt)] == [3, 2]

    async def test_bytes_order_key(self, client, products):
        stmt = Statement(NAMESPACE, SET, ["id", "sku"])
        stmt.set_order_by("sku", BinDataType.BYTES, Order.DESC)
        stmt.top_k = 3
        # Descending, so the longer value b"\x07\x01" ranks before its prefix b"\x07".
        assert [r.bins["id"] for r in await collect(client, stmt)] == [1, 2, 3]

    async def test_missing_key_ranks_last_and_k_beyond_matches_returns_all(self, client, products):
        stmt = Statement(NAMESPACE, SET, ["id", "score"])
        stmt.set_order_by("score", BinDataType.INTEGER, Order.ASC)
        stmt.top_k = 100
        ids = [r.bins["id"] for r in await collect(client, stmt)]
        # Ids 2 and 3 tie on score, and a tie with no second key breaks by digest.
        assert ids[:2] == [1, 4]
        assert set(ids[2:4]) == {2, 3}
        assert ids[4:] == [5]

    async def test_nearest_by_distance(self, client, products):
        query = Vector([0.0, 0.0], VectorElementType.FLOAT64)
        stmt = Statement(NAMESPACE, SET)
        stmt.set_operations([
            Operation.get_bin("id"),
            ExpOperation.read("d", distance(VectorDistanceMetric.EUCLIDEAN_SQUARED, query), ExpReadFlags.DEFAULT),
        ])
        stmt.set_order_by("d", BinDataType.DOUBLE, Order.ASC)
        stmt.top_k = 3
        records = await collect(client, stmt)
        assert [r.bins["id"] for r in records] == [1, 4, 3]
        assert [r.bins["d"] for r in records] == pytest.approx([0.0, 0.29, 2.0])

    @pytest.mark.parametrize(
        "configure",
        [
            lambda stmt, policy: setattr(stmt, "top_k", 0),
            lambda stmt, policy: setattr(stmt, "top_k", 1001),
            lambda stmt, policy: setattr(policy, "max_records", 10),
            lambda stmt, policy: setattr(policy, "include_bin_data", False),
            lambda stmt, policy: stmt.then_order_by("score", BinDataType.INTEGER, Order.ASC),
        ],
        ids=["k-zero", "k-too-large", "max-records", "no-bin-data", "same-bin-twice"],
    )
    async def test_invalid_top_k_is_rejected_before_sending(self, client, configure):
        stmt = Statement(NAMESPACE, SET, ["score"])
        stmt.set_order_by("score", BinDataType.INTEGER, Order.DESC)
        stmt.top_k = 5
        policy = QueryPolicy()
        configure(stmt, policy)
        with pytest.raises(AerospikeValueError) as exc_info:
            await collect(client, stmt, policy)
        assert exc_info.value.result_code == ResultCode.PARAMETER_ERROR

    async def test_background_query_rejects_top_k(self, client):
        stmt = Statement(NAMESPACE, SET)
        stmt.set_order_by("score", BinDataType.INTEGER, Order.DESC)
        stmt.top_k = 5
        with pytest.raises(AerospikeValueError) as exc_info:
            await client.query_operate(stmt, [Operation.put("seen", 1)])
        assert exc_info.value.result_code == ResultCode.PARAMETER_ERROR


class TestVectorUnsupportedCluster(TestFixtureCleanDB):
    """A cluster with a node below 8.2.1 gets a client-side refusal, not a send."""

    @pytest.fixture(autouse=True)
    def require_pre_vector_cluster(self, supports_vector):
        if supports_vector:
            pytest.skip("every node supports vectors")

    async def test_vector_write_is_refused(self, client, key):
        with pytest.raises(AerospikeValueError, match="not supported") as exc_info:
            await client.put(key, {"embedding": Vector([1.0])})
        assert exc_info.value.result_code == ResultCode.PARAMETER_ERROR

    async def test_vector_expression_is_refused(self, client, key):
        await client.put(key, {"n": 1})
        query = distance(VectorDistanceMetric.DOT_PRODUCT, Vector([1.0]))
        with pytest.raises(AerospikeValueError, match="not supported"):
            await client.operate(key, [ExpOperation.read("d", query, ExpReadFlags.DEFAULT)])

    async def test_top_k_query_is_refused(self, client):
        stmt = Statement(NAMESPACE, SET, ["n"])
        stmt.set_order_by("n", BinDataType.INTEGER, Order.DESC)
        stmt.top_k = 5
        with pytest.raises(AerospikeValueError, match="Top-K is not supported"):
            await collect(client, stmt)
