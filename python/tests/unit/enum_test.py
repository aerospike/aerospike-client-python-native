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

"""Unit tests for enums not covered by other test files."""

import subprocess
import sys

from aerospike_native import (
    BitwiseOverflowActions,
    BitwiseResizeFlags,
    BitWriteFlags,
    HLLWriteFlags,
    IndexType,
    ListOrderType,
    ListSortFlags,
    ListWriteFlags,
    MapOrder,
    MapWriteFlags,
    MapWriteMode,
    StringNumericType,
    TaskStatus,
    UDFLang,
)


class TestBitwiseOverflowActions:

    def test_variants_exist(self):
        assert BitwiseOverflowActions.FAIL is not None
        assert BitwiseOverflowActions.SATURATE is not None
        assert BitwiseOverflowActions.WRAP is not None

    def test_variants_distinct(self):
        assert BitwiseOverflowActions.FAIL != BitwiseOverflowActions.SATURATE
        assert BitwiseOverflowActions.SATURATE != BitwiseOverflowActions.WRAP
        assert BitwiseOverflowActions.FAIL != BitwiseOverflowActions.WRAP


class TestBitwiseResizeFlags:

    def test_variants_exist(self):
        assert BitwiseResizeFlags.DEFAULT is not None
        assert BitwiseResizeFlags.FROM_FRONT is not None
        assert BitwiseResizeFlags.GROW_ONLY is not None
        assert BitwiseResizeFlags.SHRINK_ONLY is not None

    def test_variants_distinct(self):
        flags = [
            BitwiseResizeFlags.DEFAULT,
            BitwiseResizeFlags.FROM_FRONT,
            BitwiseResizeFlags.GROW_ONLY,
            BitwiseResizeFlags.SHRINK_ONLY,
        ]
        for i, a in enumerate(flags):
            for b in flags[i + 1:]:
                assert a != b


class TestBitWriteFlags:

    def test_variants_exist(self):
        assert BitWriteFlags.DEFAULT is not None
        assert BitWriteFlags.CREATE_ONLY is not None
        assert BitWriteFlags.UPDATE_ONLY is not None
        assert BitWriteFlags.NO_FAIL is not None
        assert BitWriteFlags.PARTIAL is not None

    def test_variants_distinct(self):
        flags = [
            BitWriteFlags.DEFAULT,
            BitWriteFlags.CREATE_ONLY,
            BitWriteFlags.UPDATE_ONLY,
            BitWriteFlags.NO_FAIL,
            BitWriteFlags.PARTIAL,
        ]
        for i, a in enumerate(flags):
            for b in flags[i + 1:]:
                assert a != b


class TestHLLWriteFlags:

    def test_variants_exist(self):
        assert HLLWriteFlags.DEFAULT is not None
        assert HLLWriteFlags.CREATE_ONLY is not None
        assert HLLWriteFlags.UPDATE_ONLY is not None
        assert HLLWriteFlags.NO_FAIL is not None
        assert HLLWriteFlags.ALLOW_FOLD is not None

    def test_variants_distinct(self):
        flags = [
            HLLWriteFlags.DEFAULT,
            HLLWriteFlags.CREATE_ONLY,
            HLLWriteFlags.UPDATE_ONLY,
            HLLWriteFlags.NO_FAIL,
            HLLWriteFlags.ALLOW_FOLD,
        ]
        for i, a in enumerate(flags):
            for b in flags[i + 1:]:
                assert a != b


class TestIndexType:

    def test_variants_exist(self):
        assert IndexType.NUMERIC is not None
        assert IndexType.INTEGER is not None
        assert IndexType.STRING is not None
        assert IndexType.GEO2D_SPHERE is not None
        assert IndexType.BLOB is not None

    def test_variants_distinct(self):
        assert IndexType.NUMERIC != IndexType.STRING
        assert IndexType.STRING != IndexType.GEO2D_SPHERE
        assert IndexType.NUMERIC != IndexType.GEO2D_SPHERE
        assert IndexType.BLOB != IndexType.NUMERIC
        assert IndexType.BLOB != IndexType.STRING
        assert IndexType.BLOB != IndexType.GEO2D_SPHERE


class TestListOrderType:

    def test_variants_exist(self):
        assert ListOrderType.UNORDERED is not None
        assert ListOrderType.ORDERED is not None

    def test_variants_distinct(self):
        assert ListOrderType.UNORDERED != ListOrderType.ORDERED


class TestListSortFlags:

    def test_variants_exist(self):
        assert ListSortFlags.DEFAULT is not None
        assert ListSortFlags.DESCENDING is not None
        assert ListSortFlags.DROP_DUPLICATES is not None

    def test_variants_distinct(self):
        assert ListSortFlags.DEFAULT != ListSortFlags.DESCENDING
        assert ListSortFlags.DEFAULT != ListSortFlags.DROP_DUPLICATES
        assert ListSortFlags.DESCENDING != ListSortFlags.DROP_DUPLICATES


class TestListWriteFlags:

    def test_variants_exist(self):
        assert ListWriteFlags.DEFAULT is not None
        assert ListWriteFlags.ADD_UNIQUE is not None
        assert ListWriteFlags.INSERT_BOUNDED is not None
        assert ListWriteFlags.NO_FAIL is not None
        assert ListWriteFlags.PARTIAL is not None

    def test_variants_distinct(self):
        flags = [
            ListWriteFlags.DEFAULT,
            ListWriteFlags.ADD_UNIQUE,
            ListWriteFlags.INSERT_BOUNDED,
            ListWriteFlags.NO_FAIL,
            ListWriteFlags.PARTIAL,
        ]
        for i, a in enumerate(flags):
            for b in flags[i + 1:]:
                assert a != b


class TestMapOrder:

    def test_variants_exist(self):
        assert MapOrder.UNORDERED is not None
        assert MapOrder.KEY_ORDERED is not None
        assert MapOrder.KEY_VALUE_ORDERED is not None

    def test_variants_distinct(self):
        assert MapOrder.UNORDERED != MapOrder.KEY_ORDERED
        assert MapOrder.KEY_ORDERED != MapOrder.KEY_VALUE_ORDERED
        assert MapOrder.UNORDERED != MapOrder.KEY_VALUE_ORDERED


class TestMapWriteMode:

    def test_variants_exist(self):
        assert MapWriteMode.UPDATE is not None
        assert MapWriteMode.UPDATE_ONLY is not None
        assert MapWriteMode.CREATE_ONLY is not None

    def test_variants_distinct(self):
        assert MapWriteMode.UPDATE != MapWriteMode.UPDATE_ONLY
        assert MapWriteMode.UPDATE_ONLY != MapWriteMode.CREATE_ONLY
        assert MapWriteMode.UPDATE != MapWriteMode.CREATE_ONLY


class TestMapWriteFlags:

    def test_variants_exist(self):
        assert MapWriteFlags.DEFAULT is not None
        assert MapWriteFlags.CREATE_ONLY is not None
        assert MapWriteFlags.UPDATE_ONLY is not None
        assert MapWriteFlags.NO_FAIL is not None
        assert MapWriteFlags.PARTIAL is not None

    def test_variants_distinct(self):
        flags = [
            MapWriteFlags.DEFAULT,
            MapWriteFlags.CREATE_ONLY,
            MapWriteFlags.UPDATE_ONLY,
            MapWriteFlags.NO_FAIL,
            MapWriteFlags.PARTIAL,
        ]
        for i, a in enumerate(flags):
            for b in flags[i + 1 :]:
                assert a != b


class TestTaskStatus:

    def test_variants_exist(self):
        assert TaskStatus.NOT_FOUND is not None
        assert TaskStatus.IN_PROGRESS is not None
        assert TaskStatus.COMPLETE is not None

    def test_variants_distinct(self):
        assert TaskStatus.NOT_FOUND != TaskStatus.IN_PROGRESS
        assert TaskStatus.IN_PROGRESS != TaskStatus.COMPLETE
        assert TaskStatus.NOT_FOUND != TaskStatus.COMPLETE


class TestUDFLang:

    def test_lua_exists(self):
        assert UDFLang.LUA is not None


class TestResultCodeCatalog:
    """Drift guard: the Python-exposed ResultCode catalog covers every result
    code the Rust core defines, server and client side.

    The pinned lists below are the core `ResultCode` and `ClientResultCode`
    enums (aerospike-core `src/result_code.rs`). Client codes are the
    negative half of one flat namespace, the same numbering every Aerospike
    client shares. When core adds a code, regenerate the list from that enum
    and add the matching classattr in `src/enums.rs`.
    """

    CORE_SERVER_CODES = [
        "OK", "SERVER_ERROR", "KEY_NOT_FOUND_ERROR", "GENERATION_ERROR",
        "PARAMETER_ERROR", "KEY_EXISTS_ERROR", "BIN_EXISTS_ERROR",
        "CLUSTER_KEY_MISMATCH", "SERVER_MEM_ERROR", "TIMEOUT",
        "ALWAYS_FORBIDDEN", "PARTITION_UNAVAILABLE", "BIN_TYPE_ERROR",
        "RECORD_TOO_BIG", "KEY_BUSY", "SCAN_ABORT", "UNSUPPORTED_FEATURE",
        "BIN_NOT_FOUND", "DEVICE_OVERLOAD", "KEY_MISMATCH",
        "INVALID_NAMESPACE", "BIN_NAME_TOO_LONG", "FAIL_FORBIDDEN",
        "ELEMENT_NOT_FOUND", "ELEMENT_EXISTS", "ENTERPRISE_ONLY",
        "OP_NOT_APPLICABLE", "FILTERED_OUT", "LOST_CONFLICT", "INVALID_ENCODING", "XDR_KEY_BUSY",
        "MRT_BLOCKED", "MRT_VERSION_MISMATCH", "MRT_EXPIRED",
        "MRT_TOO_MANY_WRITES", "MRT_COMMITTED", "MRT_ABORTED",
        "MRT_ALREADY_LOCKED", "MRT_MONITOR_EXISTS", "QUERY_END",
        "SECURITY_NOT_SUPPORTED", "SECURITY_NOT_ENABLED",
        "SECURITY_SCHEME_NOT_SUPPORTED", "INVALID_COMMAND", "INVALID_FIELD",
        "ILLEGAL_STATE", "INVALID_USER", "USER_ALREADY_EXISTS",
        "INVALID_PASSWORD", "EXPIRED_PASSWORD", "FORBIDDEN_PASSWORD",
        "INVALID_CREDENTIAL", "EXPIRED_SESSION", "INVALID_ROLE",
        "ROLE_ALREADY_EXISTS", "INVALID_PRIVILEGE", "INVALID_ALLOWLIST",
        "QUOTAS_NOT_ENABLED", "INVALID_QUOTA", "NOT_AUTHENTICATED",
        "ROLE_VIOLATION", "NOT_ALLOWLISTED", "QUOTA_EXCEEDED",
        "UDF_BAD_RESPONSE", "BATCH_DISABLED", "BATCH_MAX_REQUESTS_EXCEEDED",
        "BATCH_QUEUES_FULL", "INVALID_GEOJSON", "INDEX_FOUND",
        "INDEX_NOT_FOUND", "INDEX_OOM", "INDEX_NOT_READABLE",
        "INDEX_GENERIC", "INDEX_NAME_MAX_LEN", "INDEX_MAX_COUNT",
        "QUERY_ABORTED", "QUERY_QUEUE_FULL", "QUERY_TIMEOUT",
        "QUERY_GENERIC", "QUERY_NETIO_ERR", "QUERY_DUPLICATE",
    ]

    # Negative, client-assigned codes with their wire values. ASYNC_QUEUE_FULL
    # (-9) is reserved in core and never produced, so it has no constant.
    CORE_CLIENT_CODES = {
        "SEED_DISCOVERY_FAILED": -27,
        "TRANSLATION_COLLISION": -26,
        "TRANSLATION_MISS": -25,
        "TXN_ALREADY_ABORTED": -19,
        "TXN_ALREADY_COMMITTED": -18,
        "TXN_FAILED": -17,
        "BATCH_FAILED": -16,
        "NO_RESPONSE": -15,
        "MAX_ERROR_RATE": -12,
        "MAX_RETRIES_EXCEEDED": -11,
        "SERIALIZE_ERROR": -10,
        "SERVER_NOT_AVAILABLE": -8,
        "NO_MORE_CONNECTIONS": -7,
        "QUERY_TERMINATED": -5,
        "SCAN_TERMINATED": -4,
        "INVALID_NODE_ERROR": -3,
        "PARSE_ERROR": -2,
        "CLIENT_ERROR": -1,
    }

    def test_every_core_client_code_is_exposed(self):
        from aerospike_native import ResultCode

        for name, value in self.CORE_CLIENT_CODES.items():
            rc = getattr(ResultCode, name)
            assert rc.name == name
            assert rc.value == value
            assert int(rc) == value
            assert rc == value
            assert repr(rc) == f"<ResultCode.{name}: {value}>"

    def test_client_code_behaves_like_its_int(self):
        from aerospike_native import ResultCode

        rc = ResultCode.TXN_FAILED
        assert rc == -17 and rc != -1 and rc != ResultCode.CLIENT_ERROR
        assert hash(rc) == hash(-17)
        assert {-17: "hit"}[rc] == "hit"
        assert str(rc) == "-17"
        assert rc.description == "Transaction failed"
        # -1 is CPython's hash error sentinel; the member must still hash
        # like the int, which CPython folds to -2.
        assert hash(ResultCode.CLIENT_ERROR) == hash(-1)

    def test_client_and_server_codes_never_collide(self):
        from aerospike_native import ResultCode

        assert ResultCode.TIMEOUT == 9 and ResultCode.TIMEOUT.value > 0
        assert ResultCode.CLIENT_ERROR != ResultCode.OK
        assert ResultCode.SERVER_NOT_AVAILABLE != ResultCode.SERVER_ERROR

    def test_every_core_server_code_is_exposed(self):
        from aerospike_native import ResultCode

        exposed = {
            n for n in dir(ResultCode) if isinstance(getattr(ResultCode, n), ResultCode)
        }
        missing = [c for c in self.CORE_SERVER_CODES if c not in exposed]
        assert not missing, f"core server codes missing from ResultCode: {missing}"

    def test_members_behave_like_int_enum_members(self):
        from aerospike_native import ResultCode

        rc = ResultCode.KEY_EXISTS_ERROR
        assert repr(rc) == "<ResultCode.KEY_EXISTS_ERROR: 5>"
        assert str(rc) == "5"
        assert f"code {rc}" == "code 5"
        assert int(rc) == 5
        assert rc.name == "KEY_EXISTS_ERROR"
        assert rc.value == 5
        assert rc.description == "Key already exists"
        assert repr(ResultCode.OK) == "<ResultCode.OK: 0>"

    def test_members_compare_and_hash_like_their_int(self):
        from aerospike_native import ResultCode

        rc = ResultCode.KEY_EXISTS_ERROR
        assert rc == 5 and 5 == rc
        assert rc != 2 and rc != ResultCode.KEY_NOT_FOUND_ERROR
        assert rc == ResultCode.KEY_EXISTS_ERROR
        assert hash(rc) == hash(5)
        assert {5: "hit"}[rc] == "hit"
        assert {rc: "hit"}[5] == "hit"
        assert (rc == "5") is False
        assert (rc == 5.0) is False

    def test_every_constant_reports_its_own_name(self):
        from aerospike_native import ResultCode

        for name in self.CORE_SERVER_CODES:
            rc = getattr(ResultCode, name)
            assert rc.name == name, f"{name} reports {rc.name!r}"
            assert repr(rc) == f"<ResultCode.{name}: {int(rc)}>"

    def test_catalog_is_complete_against_this_pin(self):
        from aerospike_native import ResultCode

        exposed = {
            n for n in dir(ResultCode) if isinstance(getattr(ResultCode, n), ResultCode)
        }
        extras = exposed - set(self.CORE_SERVER_CODES) - set(self.CORE_CLIENT_CODES)
        assert not extras, (
            f"ResultCode exposes names not in the pinned core list: {extras} "
            "— core probably added codes; regenerate the pinned list"
        )


class TestStringNumericType:

    def test_int_values(self):
        assert int(StringNumericType.ANY) == 0
        assert int(StringNumericType.INT) == 1
        assert int(StringNumericType.FLOAT) == 2


class TestTypeSlots:

    def test_import_clean_under_deprecation_errors(self):
        # Python 3.15 deprecates a type defining the same slot twice, which a
        # hand-written __int__ or __repr__ on a #[pyclass] enum does, since
        # PyO3 already generates both. Under -W error the import fails.
        result = subprocess.run(
            [sys.executable, "-W", "error::DeprecationWarning", "-c", "import aerospike_native"],
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0, result.stderr
