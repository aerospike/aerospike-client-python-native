// Copyright 2023-2026 Aerospike, Inc.
//
// Portions may be licensed to Aerospike, Inc. under one or more contributor
// license agreements WHICH ARE COMPATIBLE WITH THE APACHE LICENSE, VERSION 2.0.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations under
// the License.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

    /// Server-error detail subcodes, re-exported from the client core.
    ///
    /// When extended error detail is requested via
    /// error_detail_verbosity on a policy, the server may attach a numeric
    /// subcode to a failure, surfaced as ServerError.sub_code. Match on the
    /// (result code, subcode) pair: subcode values are scoped to their parent
    /// result code and are not globally unique. NONE (0) means no subcode.
    /// The catalog is append-only and server-version-specific; treat an
    /// unknown value as an opaque integer. Requires server 8.2.0+.
    #[gen_stub_pyclass]
    #[pyclass(name = "SubCode", module = "aerospike_native._native", frozen)]
    pub struct SubCode;

    #[gen_stub_pymethods]
    #[pymethods]
    impl SubCode {

        /// Returned when the server did not supply a subcode.
        #[classattr]
        const NONE: u32 = aerospike_core::server_error::sub_code::NONE;

        // -------------------------------------------------------
        // Pairs with ResultCode::ParameterError (4)
        // -------------------------------------------------------

        /// Per-record TTL exceeds the namespace's max-ttl.
        #[classattr]
        const PARAM_TTL_INVALID: u32 = aerospike_core::server_error::sub_code::PARAM_TTL_INVALID;
        /// Bit op offset lands past the blob (or above the proto cap).
        #[classattr]
        const PARAM_BITS_OFFSET_OUT_OF_RANGE: u32 = aerospike_core::server_error::sub_code::PARAM_BITS_OFFSET_OUT_OF_RANGE;
        /// Bit op size is out of range (e.g. zero, or too large).
        #[classattr]
        const PARAM_BITS_SIZE_OUT_OF_RANGE: u32 = aerospike_core::server_error::sub_code::PARAM_BITS_SIZE_OUT_OF_RANGE;
        /// Blob resize would exceed the maximum blob size.
        #[classattr]
        const PARAM_BITS_RESIZE_EXCEEDED: u32 = aerospike_core::server_error::sub_code::PARAM_BITS_RESIZE_EXCEEDED;
        /// Write would exceed the per-record bin-count limit (write path).
        #[classattr]
        const PARAM_BIN_COUNT_TOO_LARGE: u32 = aerospike_core::server_error::sub_code::PARAM_BIN_COUNT_TOO_LARGE;
        /// String op wire/expression args malformed or out of range.
        #[classattr]
        const PARAM_STRING_OP_PARAMS_INVALID: u32 = aerospike_core::server_error::sub_code::PARAM_STRING_OP_PARAMS_INVALID;
        /// String op code or modifier/read class mismatch on the wire path.
        #[classattr]
        const PARAM_STRING_OP_INVALID: u32 = aerospike_core::server_error::sub_code::PARAM_STRING_OP_INVALID;
        /// String context-eval path malformed.
        #[classattr]
        const PARAM_STRING_CTX_MALFORMED: u32 = aerospike_core::server_error::sub_code::PARAM_STRING_CTX_MALFORMED;
        /// String modify/read index or code-point range out of bounds.
        #[classattr]
        const PARAM_STRING_INDEX_OUT_OF_BOUNDS: u32 = aerospike_core::server_error::sub_code::PARAM_STRING_INDEX_OUT_OF_BOUNDS;
        /// String regex pattern invalid (compile / ICU failure).
        #[classattr]
        const PARAM_STRING_REGEX_INVALID: u32 = aerospike_core::server_error::sub_code::PARAM_STRING_REGEX_INVALID;
        /// String or string op argument is not valid UTF-8.
        #[classattr]
        const PARAM_STRING_UTF8_INVALID: u32 = aerospike_core::server_error::sub_code::PARAM_STRING_UTF8_INVALID;

        // -------------------------------------------------------
        // Pairs with ResultCode::PartitionUnavailable (11)
        // -------------------------------------------------------

        /// Cluster is still resolving initial partition balance at startup.
        #[classattr]
        const UNAVAIL_INITIAL_BALANCE_UNRESOLVED: u32 = aerospike_core::server_error::sub_code::UNAVAIL_INITIAL_BALANCE_UNRESOLVED;
        /// A needed replica is unavailable (likely a partition split).
        #[classattr]
        const UNAVAIL_REPLICA_UNAVAILABLE: u32 = aerospike_core::server_error::sub_code::UNAVAIL_REPLICA_UNAVAILABLE;
        /// This node is shutting down and leaving the cluster. Fail over to
        /// another node; retrying this one cannot succeed.
        #[classattr]
        const UNAVAIL_NODE_SHUTTING_DOWN: u32 = aerospike_core::server_error::sub_code::UNAVAIL_NODE_SHUTTING_DOWN;

        // -------------------------------------------------------
        // Pairs with ResultCode::UnsupportedFeature (16)
        // -------------------------------------------------------

        /// MRT attempted against a non-SC (AP) namespace.
        #[classattr]
        const UNSUPP_FEAT_MRT_REQUIRES_STRONG_CONSISTENCY: u32 = aerospike_core::server_error::sub_code::UNSUPP_FEAT_MRT_REQUIRES_STRONG_CONSISTENCY;
        /// Requested feature is unsupported in this context (generic).
        #[classattr]
        const UNSUPP_FEAT_GENERIC: u32 = aerospike_core::server_error::sub_code::UNSUPP_FEAT_GENERIC;

        // -------------------------------------------------------
        // Pairs with ResultCode::BinNotFound (17)
        // -------------------------------------------------------

        /// HLL op needs an existing bin and can't auto-create one.
        #[classattr]
        const BIN_NOT_FOUND_HLL_CANNOT_CREATE_WITH_OP: u32 = aerospike_core::server_error::sub_code::BIN_NOT_FOUND_HLL_CANNOT_CREATE_WITH_OP;
        /// String modify on a missing bin (non-NO_FAIL path).
        #[classattr]
        const BIN_NOT_FOUND_STRING_VALUE_NOT_FOUND: u32 = aerospike_core::server_error::sub_code::BIN_NOT_FOUND_STRING_VALUE_NOT_FOUND;

        // -------------------------------------------------------
        // Pairs with ResultCode::BinNameTooLong (21)
        // -------------------------------------------------------

        /// Write would exceed the per-record bin-count limit (UDF path).
        #[classattr]
        const BIN_NAME_COUNT_TOO_LARGE: u32 = aerospike_core::server_error::sub_code::BIN_NAME_COUNT_TOO_LARGE;

        // -------------------------------------------------------
        // Pairs with ResultCode::FailForbidden (22)
        // -------------------------------------------------------

        /// Write bounced by an XDR ship filter at the destination.
        #[classattr]
        const FORBID_XDR_FILTER_BLOCKED: u32 = aerospike_core::server_error::sub_code::FORBID_XDR_FILTER_BLOCKED;
        /// Set-level record-count stop-writes limit reached.
        #[classattr]
        const FORBID_SET_COUNT_STOP_WRITES: u32 = aerospike_core::server_error::sub_code::FORBID_SET_COUNT_STOP_WRITES;
        /// Set-level size stop-writes limit reached.
        #[classattr]
        const FORBID_SET_SIZE_STOP_WRITES: u32 = aerospike_core::server_error::sub_code::FORBID_SET_SIZE_STOP_WRITES;
        /// Writes stopped due to cluster clock skew.
        #[classattr]
        const FORBID_CLOCK_SKEW_STOP_WRITES: u32 = aerospike_core::server_error::sub_code::FORBID_CLOCK_SKEW_STOP_WRITES;
        /// `REPLACE` / `CREATE_OR_REPLACE` forbidden while resolving conflicts.
        #[classattr]
        const FORBID_REPLACE_CONFLICT_RESOLVING: u32 = aerospike_core::server_error::sub_code::FORBID_REPLACE_CONFLICT_RESOLVING;
        /// Write forbidden because the set/namespace is mid-truncate.
        #[classattr]
        const FORBID_TRUNCATED: u32 = aerospike_core::server_error::sub_code::FORBID_TRUNCATED;
        // Server subcodes 7 and 9 in this family are retired (masking violations
        // return ROLE_VIOLATION, not FORBIDDEN) and are intentionally not declared.
        /// Non-durable delete forbidden (would violate durability).
        #[classattr]
        const FORBID_DURABILITY_VIOLATION: u32 = aerospike_core::server_error::sub_code::FORBID_DURABILITY_VIOLATION;

        // -------------------------------------------------------
        // Pairs with ResultCode::OpNotApplicable (26)
        // -------------------------------------------------------

        /// List index is outside the current element range.
        #[classattr]
        const OPNOT_CDT_INDEX_OUT_OF_BOUNDS: u32 = aerospike_core::server_error::sub_code::OPNOT_CDT_INDEX_OUT_OF_BOUNDS;
        /// Requested rank is past the current population.
        #[classattr]
        const OPNOT_CDT_RANK_OUT_OF_BOUNDS: u32 = aerospike_core::server_error::sub_code::OPNOT_CDT_RANK_OUT_OF_BOUNDS;
        /// Insert would exceed an ordered+bounded list's cap.
        #[classattr]
        const OPNOT_CDT_BOUNDED_LIST_OVERFLOW: u32 = aerospike_core::server_error::sub_code::OPNOT_CDT_BOUNDED_LIST_OVERFLOW;
        /// HLL op needs `index_bits` but the sketch has none set.
        #[classattr]
        const OPNOT_HLL_INDEX_BITS_UNSET: u32 = aerospike_core::server_error::sub_code::OPNOT_HLL_INDEX_BITS_UNSET;
        /// Union needs to reduce `index_bits` but folding isn't allowed.
        #[classattr]
        const OPNOT_HLL_CANNOT_REDUCE_INDEX_BITS: u32 = aerospike_core::server_error::sub_code::OPNOT_HLL_CANNOT_REDUCE_INDEX_BITS;
        /// As above, for the minhash dimension.
        #[classattr]
        const OPNOT_HLL_CANNOT_REDUCE_MINHASH_BITS: u32 = aerospike_core::server_error::sub_code::OPNOT_HLL_CANNOT_REDUCE_MINHASH_BITS;
        /// Fold blocked because the sketch carries minhash bits.
        #[classattr]
        const OPNOT_HLL_CANNOT_FOLD_MINHASH: u32 = aerospike_core::server_error::sub_code::OPNOT_HLL_CANNOT_FOLD_MINHASH;
        /// Fold target `index_bits` >= current (fold can only reduce).
        #[classattr]
        const OPNOT_HLL_FOLD_INDEX_BITS_TOO_LARGE: u32 = aerospike_core::server_error::sub_code::OPNOT_HLL_FOLD_INDEX_BITS_TOO_LARGE;
        /// Intersect inputs have mismatched minhash parameters.
        #[classattr]
        const OPNOT_HLL_INTERSECT_MINHASH_MISMATCH: u32 = aerospike_core::server_error::sub_code::OPNOT_HLL_INTERSECT_MINHASH_MISMATCH;
        /// String to numeric conversion failed.
        #[classattr]
        const OPNOT_STRING_CONVERSION_FAILED: u32 = aerospike_core::server_error::sub_code::OPNOT_STRING_CONVERSION_FAILED;
        /// Source blob/string is not valid UTF-8 for an `OpNotApplicable` path.
        #[classattr]
        const OPNOT_STRING_UTF8_INVALID: u32 = aerospike_core::server_error::sub_code::OPNOT_STRING_UTF8_INVALID;
        /// Regex evaluation exceeded its resource budget (server 8.2+).
        #[classattr]
        const OPNOT_STRING_REGEX_LIMIT_EXCEEDED: u32 = aerospike_core::server_error::sub_code::OPNOT_STRING_REGEX_LIMIT_EXCEEDED;
        /// The string is not valid base64 — a length that is not a multiple of 4, a
        /// character outside the alphabet, or misplaced `=` padding.
        #[classattr]
        const OPNOT_STRING_B64_INVALID: u32 = aerospike_core::server_error::sub_code::OPNOT_STRING_B64_INVALID;

        // -------------------------------------------------------
        // Pairs with ResultCode::MrtBlocked (120)
        // -------------------------------------------------------

        /// Record is provisionally locked by another MRT.
        #[classattr]
        const MRT_BLOCKED_RECORD_LOCKED: u32 = aerospike_core::server_error::sub_code::MRT_BLOCKED_RECORD_LOCKED;
        /// Op belongs to a different MRT than the one holding the lock.
        #[classattr]
        const MRT_BLOCKED_ID_MISMATCH: u32 = aerospike_core::server_error::sub_code::MRT_BLOCKED_ID_MISMATCH;
    }

    /// Verbosity levels for error_detail_verbosity on a policy.
    ///
    /// NONE (0) requests no extended detail (default). SUBCODE (1)
    /// requests the numeric subcode; MESSAGE (2) adds the server message;
    /// EXPRESSION_TRACE (3) adds an expression trace on expression build
    /// failures.
    /// Higher levels are supersets. Requires server 8.2.0+; older servers
    /// ignore the request.
    #[gen_stub_pyclass]
    #[pyclass(name = "ErrorDetailVerbosity", module = "aerospike_native._native", frozen)]
    pub struct ErrorDetailVerbosity;

    #[gen_stub_pymethods]
    #[pymethods]
    impl ErrorDetailVerbosity {
        #[classattr]
        const NONE: u8 = 0;
        #[classattr]
        const SUBCODE: u8 = 1;
        #[classattr]
        const MESSAGE: u8 = 2;
        #[classattr]
        const EXPRESSION_TRACE: u8 = 3;
    }

    /// A structured expression build trace, surfaced on ServerError.exp_trace
    /// at error_detail_verbosity 3 when the server fails to build an expression
    /// (a filter_expression, or an exp_read / exp_write operation).
    ///
    /// Every field is optional: the server caps the detail payload and drops
    /// snippet first, then path, under a tight byte budget, so treat any field
    /// as possibly absent. Expression build failures carry
    /// ResultCode.PARAMETER_ERROR and no subcode; this trace is purely additive
    /// diagnostic detail and never changes the result code, subcode, or message.
    /// byte_offset indexes the msgpack expression payload the client sent;
    /// ael_offset / ael_span index AEL source text (a different coordinate
    /// space, reserved for a future server branch). Requires a server build that
    /// emits the trace.
    #[gen_stub_pyclass]
    #[pyclass(name = "ExpressionTrace", module = "aerospike_native._native", frozen, eq, from_py_object)]
    #[derive(Clone, PartialEq)]
    pub struct ExpressionTrace {
        /// Phase that failed: PHASE_BUILD or PHASE_EVAL; None when absent.
        #[pyo3(get)]
        pub phase: Option<u32>,
        /// Byte offset into the msgpack expression payload of the failing element.
        #[pyo3(get)]
        pub byte_offset: Option<u32>,
        /// The failing op name (pre-rendered server-side).
        #[pyo3(get)]
        pub op: Option<String>,
        /// True nesting depth of the fault (accurate even when path was truncated).
        #[pyo3(get)]
        pub depth: Option<u32>,
        /// Op-name chain from root to fault; may contain a "..." truncation
        /// sentinel when nesting exceeded the server's path-frame cap.
        #[pyo3(get)]
        pub path: Option<Vec<String>>,
        /// Human-only rendered snippet of the failing element (dropped first
        /// under a tight byte budget).
        #[pyo3(get)]
        pub snippet: Option<String>,
        /// Source language: LANG_MSGPACK (the default) or LANG_AEL.
        #[pyo3(get)]
        pub lang: Option<u32>,
        /// Char offset into AEL source text (reserved for the AEL branch).
        #[pyo3(get)]
        pub ael_offset: Option<u32>,
        /// Byte width of the offending AEL source region (reserved for the AEL branch).
        #[pyo3(get)]
        pub ael_span: Option<u32>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl ExpressionTrace {
        /// The expression build failed.
        #[classattr]
        const PHASE_BUILD: u32 = aerospike_core::server_error::EXP_TRACE_PHASE_BUILD;
        /// Expression evaluation failed (reserved for a future server branch).
        #[classattr]
        const PHASE_EVAL: u32 = aerospike_core::server_error::EXP_TRACE_PHASE_EVAL;
        /// The msgpack source language (the implied default).
        #[classattr]
        const LANG_MSGPACK: u32 = aerospike_core::server_error::EXP_TRACE_LANG_MSGPACK;
        /// The AEL DSL source language (reserved for a future server branch).
        #[classattr]
        const LANG_AEL: u32 = aerospike_core::server_error::EXP_TRACE_LANG_AEL;

        fn __repr__(&self) -> String {
            format!(
                "ExpressionTrace(phase={:?}, byte_offset={:?}, op={:?}, depth={:?}, \
                 path={:?}, snippet={:?}, lang={:?}, ael_offset={:?}, ael_span={:?})",
                self.phase, self.byte_offset, self.op, self.depth, self.path,
                self.snippet, self.lang, self.ael_offset, self.ael_span,
            )
        }
    }

    impl ExpressionTrace {
        /// Build the Python wrapper from the client-core trace struct.
        pub fn from_core(t: &aerospike_core::server_error::ExpressionTrace) -> Self {
            ExpressionTrace {
                phase: t.phase,
                byte_offset: t.byte_offset,
                op: t.op.clone(),
                depth: t.depth,
                path: t.path.clone(),
                snippet: t.snippet.clone(),
                lang: t.lang,
                ael_offset: t.ael_offset,
                ael_span: t.ael_span,
            }
        }
    }
