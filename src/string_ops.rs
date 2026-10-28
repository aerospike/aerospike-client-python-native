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

//! String operation policy / flag types exposed to Python.
//!
//! Mirrors the pattern in [`crate::cdt`] for `BitWriteFlags`, `MapWriteFlags`,
//! and `ListReturnType`. The three Python classes here are pure tag types —
//! zero per-op cost (the values are class-level constants resolved at module
//! init).
//!
//! Per the string-ops spec (§3.4–§3.6):
//!
//! - `StringWriteFlags::{DEFAULT, CREATE_ONLY, UPDATE_ONLY, NO_FAIL}` —
//!   see the enum docs for the per-op validity rules.
//! - `StringRegexFlags::{DEFAULT, CASE_INSENSITIVE, MULTILINE, DOTALL,
//!   UNIX_LINES, GLOBAL}` — flag name `DOTALL` is one word (matching
//!   Python's stdlib `re.DOTALL`); rust-core spells the same flag
//!   `DOT_ALL`.
//! - `StringNumericType::{ANY, INT, FLOAT}` — passed as optional second arg
//!   to `IS_NUMERIC` (sub-op id 10).
//!
//! Requires Aerospike Server 8.2.0+ at runtime; these tag types are version-
//! independent and can be constructed against any cluster.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};

use crate::cdt::CTX;
use crate::operations::OperationType;

////////////////////////////////////////////////////////////////////////////////////////////
//
//  StringWriteFlags
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Per-operation write flags for string modify ops. Combine with bitwise OR.
///
/// ``CREATE_ONLY`` and ``UPDATE_ONLY`` are mutually exclusive; sending both
/// is a server ``ParameterError``.
#[gen_stub_pyclass_enum(module = "_native")]
#[pyclass(from_py_object, name = "StringWriteFlags", module = "_native")]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringWriteFlags {
    /// Default. Allow create or update.
    #[pyo3(name = "DEFAULT")]
    Default = 0,
    /// Apply only if the bin does not already exist; a live bin raises
    /// ``BinExistsError`` (suppressible with ``NO_FAIL``). Valid only on the
    /// additive ops (insert, overwrite, concat, append, prepend, pad_start,
    /// pad_end, repeat) and never with a CTX path — either misuse is a
    /// server ``ParameterError``, which ``NO_FAIL`` does not suppress.
    #[pyo3(name = "CREATE_ONLY")]
    CreateOnly = 1,
    /// Apply only to an existing bin: on a missing bin the op is a silent
    /// no-op instead of creating it. Valid on all string modify ops.
    #[pyo3(name = "UPDATE_ONLY")]
    UpdateOnly = 2,
    /// Suppress in-op execution failures — e.g. the ``BinExistsError`` from
    /// ``CREATE_ONLY`` on a live bin, or ``OpNotApplicable`` from an
    /// unreachable CTX path: the op becomes a no-op and the bin keeps its
    /// current value. Does NOT suppress wrong-bin-type or invalid
    /// UTF-8 errors, and has no effect on missing bins — a missing bin is
    /// never an error for string ops (additive ops create it from empty,
    /// the other modifies no-op), with or without this flag.
    #[pyo3(name = "NO_FAIL")]
    NoFail = 4,
}

impl From<StringWriteFlags> for u8 {
    #[inline]
    fn from(flags: StringWriteFlags) -> Self {
        flags as u8
    }
}

#[pymethods]
impl StringWriteFlags {
    /// Combine flags with bitwise OR. Returns the wire byte as int so combined
    /// values can be passed wherever a single flag is accepted.
    fn __or__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_write_flags_from_py(other)?;
        Ok(a | b)
    }

    /// Right-hand bitwise OR (e.g. `int | StringWriteFlags.X`).
    fn __ror__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_write_flags_from_py(other)?;
        Ok(a | b)
    }

    /// Bitwise AND.
    fn __and__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_write_flags_from_py(other)?;
        Ok(a & b)
    }

    /// Right-hand bitwise AND.
    fn __rand__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_write_flags_from_py(other)?;
        Ok(b & a)
    }

    /// Bitwise XOR.
    fn __xor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_write_flags_from_py(other)?;
        Ok(a ^ b)
    }

    /// Right-hand bitwise XOR.
    fn __rxor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_write_flags_from_py(other)?;
        Ok(b ^ a)
    }

    /// Bitwise NOT (masked to u8 flag width).
    fn __invert__(&self) -> u8 {
        !u8::from(*self)
    }

    /// Equality / ordering against another ``StringWriteFlags`` *or* an
    /// ``int`` bitmask. Honors the ``IntEnum`` runtime contract promised by
    /// the generated stubs.
    fn __richcmp__(
        &self,
        other: &Bound<'_, PyAny>,
        op: pyo3::class::basic::CompareOp,
    ) -> pyo3::PyResult<bool> {
        let a = u8::from(*self) as i64;
        let b = match string_write_flags_from_py(other) {
            Ok(v) => v as i64,
            Err(_) => {
                return Ok(matches!(op, pyo3::class::basic::CompareOp::Ne));
            }
        };
        Ok(match op {
            pyo3::class::basic::CompareOp::Eq => a == b,
            pyo3::class::basic::CompareOp::Ne => a != b,
            pyo3::class::basic::CompareOp::Lt => a < b,
            pyo3::class::basic::CompareOp::Le => a <= b,
            pyo3::class::basic::CompareOp::Gt => a > b,
            pyo3::class::basic::CompareOp::Ge => a >= b,
        })
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// Extract write flags as u8 from `StringWriteFlags` or `int` (bitmask).
/// Used by both `StringWriteFlags`' bitwise dunders and by the
/// `StringOperation` factory methods that accept a `flags=` kwarg.
#[inline]
pub(crate) fn string_write_flags_from_py(ob: &Bound<'_, PyAny>) -> PyResult<u8> {
    if let Ok(f) = ob.extract::<StringWriteFlags>() {
        return Ok(u8::from(f));
    }
    if let Ok(i) = ob.extract::<i64>() {
        return Ok(i as u8);
    }
    Err(PyValueError::new_err("flags must be StringWriteFlags or int"))
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  StringRegexFlags
//
////////////////////////////////////////////////////////////////////////////////////////////

/// ICU regex flags for `regex_compare` and `regex_replace`. Combine with
/// bitwise OR. Default is no flags.
#[gen_stub_pyclass_enum(module = "_native")]
#[pyclass(from_py_object, name = "StringRegexFlags", module = "_native")]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringRegexFlags {
    /// No flags.
    #[pyo3(name = "DEFAULT")]
    Default = 0,
    /// Case-insensitive matching.
    #[pyo3(name = "CASE_INSENSITIVE")]
    CaseInsensitive = 1,
    /// Multi-line: `^` and `$` match the start and end of any line.
    #[pyo3(name = "MULTILINE")]
    Multiline = 2,
    /// `.` matches any character including line terminators. Spelled
    /// `DOTALL` (one word) to match Python's stdlib `re.DOTALL`;
    /// rust-core spells the same flag `DOT_ALL`.
    #[pyo3(name = "DOTALL")]
    DotAll = 4,
    /// Only `\n` is treated as a line terminator (Unix-style line endings).
    #[pyo3(name = "UNIX_LINES")]
    UnixLines = 8,
    /// `regex_replace` only — replace every match (default replaces only
    /// the first match).
    #[pyo3(name = "GLOBAL")]
    Global = 16,
}

impl From<StringRegexFlags> for u8 {
    #[inline]
    fn from(flags: StringRegexFlags) -> Self {
        flags as u8
    }
}

#[pymethods]
impl StringRegexFlags {
    /// Combine flags with bitwise OR. Returns the wire byte as int so combined
    /// values can be passed wherever a single flag is accepted.
    fn __or__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_regex_flags_from_py(other)?;
        Ok(a | b)
    }

    /// Right-hand bitwise OR (e.g. `int | StringRegexFlags.X`).
    fn __ror__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_regex_flags_from_py(other)?;
        Ok(a | b)
    }

    /// Bitwise AND.
    fn __and__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_regex_flags_from_py(other)?;
        Ok(a & b)
    }

    /// Right-hand bitwise AND.
    fn __rand__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_regex_flags_from_py(other)?;
        Ok(b & a)
    }

    /// Bitwise XOR.
    fn __xor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_regex_flags_from_py(other)?;
        Ok(a ^ b)
    }

    /// Right-hand bitwise XOR.
    fn __rxor__(&self, other: &Bound<'_, PyAny>) -> PyResult<u8> {
        let a = u8::from(*self);
        let b = string_regex_flags_from_py(other)?;
        Ok(b ^ a)
    }

    /// Bitwise NOT (masked to u8 flag width).
    fn __invert__(&self) -> u8 {
        !u8::from(*self)
    }

    fn __richcmp__(
        &self,
        other: &Bound<'_, PyAny>,
        op: pyo3::class::basic::CompareOp,
    ) -> pyo3::PyResult<bool> {
        let a = u8::from(*self) as i64;
        let b = match string_regex_flags_from_py(other) {
            Ok(v) => v as i64,
            Err(_) => {
                return Ok(matches!(op, pyo3::class::basic::CompareOp::Ne));
            }
        };
        Ok(match op {
            pyo3::class::basic::CompareOp::Eq => a == b,
            pyo3::class::basic::CompareOp::Ne => a != b,
            pyo3::class::basic::CompareOp::Lt => a < b,
            pyo3::class::basic::CompareOp::Le => a <= b,
            pyo3::class::basic::CompareOp::Gt => a > b,
            pyo3::class::basic::CompareOp::Ge => a >= b,
        })
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// Extract regex flags as u8 from `StringRegexFlags` or `int` (bitmask).
#[inline]
pub(crate) fn string_regex_flags_from_py(ob: &Bound<'_, PyAny>) -> PyResult<u8> {
    if let Ok(f) = ob.extract::<StringRegexFlags>() {
        return Ok(u8::from(f));
    }
    if let Ok(i) = ob.extract::<i64>() {
        return Ok(i as u8);
    }
    Err(PyValueError::new_err("regex_flags must be StringRegexFlags or int"))
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  StringNumericType
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Numeric-type filter for `StringOperation.is_numeric`. Default `ANY`
/// matches integers or floats; restrict to one or the other with `INT` /
/// `FLOAT`.
#[gen_stub_pyclass_enum(module = "_native")]
#[pyclass(from_py_object, name = "StringNumericType", module = "_native")]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringNumericType {
    /// Match either an integer or a floating-point number.
    #[pyo3(name = "ANY")]
    Any = 0,
    /// Match only integers.
    #[pyo3(name = "INT")]
    Int = 1,
    /// Match only floating-point numbers.
    #[pyo3(name = "FLOAT")]
    Float = 2,
}

impl From<StringNumericType> for u8 {
    #[inline]
    fn from(t: StringNumericType) -> Self {
        t as u8
    }
}

#[pymethods]
impl StringNumericType {
    fn __richcmp__(
        &self,
        other: &Bound<'_, PyAny>,
        op: pyo3::class::basic::CompareOp,
    ) -> pyo3::PyResult<bool> {
        let a = u8::from(*self) as i64;
        let b = if let Ok(t) = other.extract::<StringNumericType>() {
            u8::from(t) as i64
        } else if let Ok(i) = other.extract::<i64>() {
            i
        } else {
            return Ok(matches!(op, pyo3::class::basic::CompareOp::Ne));
        };
        Ok(match op {
            pyo3::class::basic::CompareOp::Eq => a == b,
            pyo3::class::basic::CompareOp::Ne => a != b,
            pyo3::class::basic::CompareOp::Lt => a < b,
            pyo3::class::basic::CompareOp::Le => a <= b,
            pyo3::class::basic::CompareOp::Gt => a > b,
            pyo3::class::basic::CompareOp::Ge => a >= b,
        })
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  StringOperation
//
//  Server-side string operations (server 8.2.0+). Spec sub-ops 0..16 (read,
//  STRING_READ op-type 17) and 50..66 (modify, STRING_MODIFY op-type 18),
//  plus the top-level TO_STRING op-type 19. Wraps the rust-core surface in
//  aerospike-core/src/operations/string.rs.
//
//  Signature shape (per spec §3.1 / §3.2):
//    - Read ops:   (bin, ...op_args, *, ctx=None)
//    - Modify ops: (bin, ...op_args, *, flags=0, ctx=None)
//    - to_string:  (bin) — no ctx (top-level wire op, no payload)
//
//  Performance notes:
//    - Each factory call allocates one String (bin) + 0..N args. Per-op
//      cost matches existing ListOperation/BitOperation patterns.
//    - The variant carries owned data; the convert path borrows from it
//      via the standard ctx-aware converter (no extra pre-pass needed
//      because the args are all already in owned-Rust form).
//
////////////////////////////////////////////////////////////////////////////////////////////

/// String bin operations (server 8.2.0+). Use these to inspect or modify
/// string bins via the client's ``operate()`` method.
///
/// Index orientation is left-to-right with codepoint addressing. Negative
/// indexes count from the end of the string (-1 = last codepoint).
/// Out-of-bounds indexes are clamped to the valid range.
///
/// CTX navigation: every factory (except ``to_string``) accepts an optional
/// trailing ``ctx`` argument selecting a string element nested inside a
/// list/map bin. With ``ctx=None`` the op targets the bin itself.
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, subclass)]
#[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
#[derive(Clone, Debug)]
pub struct StringOperation {
    pub(crate) op: OperationType,
    pub(crate) ctx: Option<Vec<CTX>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl StringOperation {
    /// ``True`` when re-sending this operation after an in-doubt attempt
    /// leaves the record as one application would. Reads and ``to_string``
    /// are idempotent; every modify operation is not. See
    /// :attr:`Operation.is_idempotent` for the rule.
    #[getter]
    pub fn is_idempotent(&self) -> bool {
        self.op.is_idempotent()
    }

    // -----------------------------------------------------------------
    // Read operations (STRING_READ, op-type 17, sub-ops 0..16)
    // -----------------------------------------------------------------

    /// Codepoint count (NOT byte count — use ``byte_length`` for bytes).
    #[staticmethod]
    #[pyo3(signature = (bin, *, ctx=None))]
    pub fn strlen(bin: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringStrlen(bin), ctx }
    }

    /// Substring from codepoint ``start`` to the end (when ``end`` is None),
    /// or the half-open range ``[start, end)`` (when ``end`` is given).
    /// Negative indexes count from the end; out-of-bounds clamp.
    #[staticmethod]
    #[pyo3(signature = (bin, start, end=None, *, ctx=None))]
    pub fn substr(bin: String, start: i64, end: Option<i64>, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringSubstr(bin, start, end), ctx }
    }

    /// Codepoint at ``index`` as a one-codepoint string. Negative = from end.
    #[staticmethod]
    #[pyo3(signature = (bin, index, *, ctx=None))]
    pub fn char_at(bin: String, index: i64, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringCharAt(bin, index), ctx }
    }

    /// First-match codepoint index of ``needle`` (returns -1 if absent), or
    /// the N-th-match index when ``occurrence`` is given (1 = first match,
    /// -1 = last match).
    #[staticmethod]
    #[pyo3(signature = (bin, needle, occurrence=None, *, ctx=None))]
    pub fn find(bin: String, needle: String, occurrence: Option<i64>, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringFind(bin, needle, occurrence), ctx }
    }

    /// True if the bin contains ``needle`` as a substring.
    #[staticmethod]
    #[pyo3(signature = (bin, needle, *, ctx=None))]
    pub fn contains(bin: String, needle: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringContains(bin, needle), ctx }
    }

    /// True if the bin starts with ``prefix``.
    #[staticmethod]
    #[pyo3(signature = (bin, prefix, *, ctx=None))]
    pub fn starts_with(bin: String, prefix: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringStartsWith(bin, prefix), ctx }
    }

    /// True if the bin ends with ``suffix``.
    #[staticmethod]
    #[pyo3(signature = (bin, suffix, *, ctx=None))]
    pub fn ends_with(bin: String, suffix: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringEndsWith(bin, suffix), ctx }
    }

    /// Parse the bin as an integer. Returns OP_NOT_APPLICABLE (subcode
    /// ``OPNOT_STRING_CONVERSION_FAILED``) if unparseable.
    #[staticmethod]
    #[pyo3(signature = (bin, *, ctx=None))]
    pub fn to_integer(bin: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringToInteger(bin), ctx }
    }

    /// Parse the bin as a float (f64). Returns OP_NOT_APPLICABLE (subcode
    /// ``OPNOT_STRING_CONVERSION_FAILED``) if unparseable.
    #[staticmethod]
    #[pyo3(signature = (bin, *, ctx=None))]
    pub fn to_double(bin: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringToDouble(bin), ctx }
    }

    /// UTF-8 byte length (differs from ``strlen`` for non-ASCII content).
    #[staticmethod]
    #[pyo3(signature = (bin, *, ctx=None))]
    pub fn byte_length(bin: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringByteLength(bin), ctx }
    }

    /// True if the bin contains a valid number. Pass ``numeric_type`` to
    /// restrict to integer-only (``StringNumericType.INT``) or float-only
    /// (``StringNumericType.FLOAT``).
    #[staticmethod]
    #[pyo3(signature = (bin, numeric_type=None, *, ctx=None))]
    pub fn is_numeric(bin: String, numeric_type: Option<StringNumericType>, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringIsNumeric(bin, numeric_type), ctx }
    }

    /// True if every cased codepoint is uppercase.
    #[staticmethod]
    #[pyo3(signature = (bin, *, ctx=None))]
    pub fn is_upper(bin: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringIsUpper(bin), ctx }
    }

    /// True if every cased codepoint is lowercase.
    #[staticmethod]
    #[pyo3(signature = (bin, *, ctx=None))]
    pub fn is_lower(bin: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringIsLower(bin), ctx }
    }

    /// Return the UTF-8 bytes of the string as a blob.
    #[staticmethod]
    #[pyo3(signature = (bin, *, ctx=None))]
    pub fn to_blob(bin: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringToBlob(bin), ctx }
    }

    /// Split the bin into a list of strings. With ``separator=None`` returns
    /// one element per codepoint.
    #[staticmethod]
    #[pyo3(signature = (bin, separator=None, *, ctx=None))]
    pub fn split(bin: String, separator: Option<String>, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringSplit(bin, separator), ctx }
    }

    /// Treat the bin as base64-encoded text and return the decoded bytes.
    /// Returns OP_NOT_APPLICABLE (subcode ``OPNOT_STRING_B64_INVALID``) if
    /// the bin is not valid base64.
    #[staticmethod]
    #[pyo3(signature = (bin, *, ctx=None))]
    pub fn b64_decode(bin: String, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringB64Decode(bin), ctx }
    }

    /// True if the ICU regex ``pattern`` matches the bin. ``flags`` is an
    /// OR-combined ``StringRegexFlags`` bitmask (or 0 for no flags).
    #[staticmethod]
    #[pyo3(signature = (bin, pattern, flags=0, *, ctx=None))]
    pub fn regex_compare(bin: String, pattern: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringRegexCompare(bin, pattern, flags), ctx }
    }

    // -----------------------------------------------------------------
    // Modify operations (STRING_MODIFY, op-type 18, sub-ops 50..66)
    // -----------------------------------------------------------------

    /// Splice ``value`` into the bin at codepoint ``index``. Negative index
    /// counts from the end.
    #[staticmethod]
    #[pyo3(signature = (bin, index, value, *, flags=0, ctx=None))]
    pub fn insert(bin: String, index: i64, value: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringInsert(bin, index, value, flags), ctx }
    }

    /// Overwrite codepoints starting at ``index`` with ``value``. May extend
    /// past the original length.
    #[staticmethod]
    #[pyo3(signature = (bin, index, value, *, flags=0, ctx=None))]
    pub fn overwrite(bin: String, index: i64, value: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringOverwrite(bin, index, value, flags), ctx }
    }

    /// Append ``value`` to the bin. Accepts either a single string or a
    /// list of strings; the list form appends each element in order.
    #[staticmethod]
    #[pyo3(signature = (bin, value, *, flags=0, ctx=None))]
    pub fn concat(bin: String, value: &Bound<'_, PyAny>, flags: u8, ctx: Option<Vec<CTX>>) -> PyResult<Self> {
        // Accept either str or list[str]; the wire format is always list-of-strings.
        let values: Vec<String> = if let Ok(s) = value.extract::<String>() {
            vec![s]
        } else if let Ok(v) = value.extract::<Vec<String>>() {
            v
        } else {
            return Err(PyValueError::new_err("concat value must be str or list[str]"));
        };
        Ok(StringOperation { op: OperationType::StringConcat(bin, values, flags), ctx })
    }

    /// Append ``value`` to the end of the bin. Single-value form (server sub-op
    /// 67); use ``concat`` for the list form.
    #[staticmethod]
    #[pyo3(signature = (bin, value, *, flags=0, ctx=None))]
    pub fn append(bin: String, value: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringAppend(bin, value, flags), ctx }
    }

    /// Prepend ``value`` to the start of the bin. Distinct from ``insert(0, …)``
    /// — this is the server's dedicated prepend sub-op (68).
    #[staticmethod]
    #[pyo3(signature = (bin, value, *, flags=0, ctx=None))]
    pub fn prepend(bin: String, value: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringPrepend(bin, value, flags), ctx }
    }

    /// Remove the half-open codepoint range ``[start, end)`` from the bin,
    /// or everything from ``start`` through the end when ``end`` is None —
    /// ``snip("s", 5)`` truncates ``"hello world"`` to ``"hello"``.
    ///
    /// ``flags`` require an explicit ``end``. The server reads the snip
    /// arguments by position — ``start``, ``end``, then flags — so the 1-arg
    /// form is packed as ``[53, start]`` with no flags element; a 2-element
    /// ``[start, flags]`` wire would land the flags in the ``end`` slot and
    /// silently snip the empty range ``[start, 0)``.
    #[staticmethod]
    #[pyo3(signature = (bin, start, end=None, *, flags=0, ctx=None))]
    pub fn snip(bin: String, start: i64, end: Option<i64>, flags: u8, ctx: Option<Vec<CTX>>) -> PyResult<Self> {
        if end.is_none() && flags != 0 {
            return Err(PyValueError::new_err(
                "snip flags require an explicit end: the server parses snip args by position, \
                 so flags cannot be sent with the 1-arg truncate-to-end form",
            ));
        }
        Ok(StringOperation { op: OperationType::StringSnip(bin, start, end, flags), ctx })
    }

    /// Replace the first occurrence of ``needle`` with ``replacement``.
    #[staticmethod]
    #[pyo3(signature = (bin, needle, replacement, *, flags=0, ctx=None))]
    pub fn replace(bin: String, needle: String, replacement: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringReplace(bin, needle, replacement, flags), ctx }
    }

    /// Replace every occurrence of ``needle`` with ``replacement``.
    #[staticmethod]
    #[pyo3(signature = (bin, needle, replacement, *, flags=0, ctx=None))]
    pub fn replace_all(bin: String, needle: String, replacement: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringReplaceAll(bin, needle, replacement, flags), ctx }
    }

    /// Uppercase the bin in place.
    #[staticmethod]
    #[pyo3(signature = (bin, *, flags=0, ctx=None))]
    pub fn upper(bin: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringUpper(bin, flags), ctx }
    }

    /// Lowercase the bin in place.
    #[staticmethod]
    #[pyo3(signature = (bin, *, flags=0, ctx=None))]
    pub fn lower(bin: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringLower(bin, flags), ctx }
    }

    /// Apply a locale-independent case fold (lowercase). Useful for
    /// normalized comparison keys.
    #[staticmethod]
    #[pyo3(signature = (bin, *, flags=0, ctx=None))]
    pub fn case_fold(bin: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringCaseFold(bin, flags), ctx }
    }

    /// Normalize the bin to Unicode NFC form. Already-normalized strings
    /// are unchanged.
    #[staticmethod]
    #[pyo3(signature = (bin, *, flags=0, ctx=None))]
    pub fn normalize_nfc(bin: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringNormalizeNfc(bin, flags), ctx }
    }

    /// Strip whitespace from the start of the bin.
    #[staticmethod]
    #[pyo3(signature = (bin, *, flags=0, ctx=None))]
    pub fn trim_start(bin: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringTrimStart(bin, flags), ctx }
    }

    /// Strip whitespace from the end of the bin.
    #[staticmethod]
    #[pyo3(signature = (bin, *, flags=0, ctx=None))]
    pub fn trim_end(bin: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringTrimEnd(bin, flags), ctx }
    }

    /// Strip whitespace from both ends of the bin.
    #[staticmethod]
    #[pyo3(signature = (bin, *, flags=0, ctx=None))]
    pub fn trim(bin: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringTrim(bin, flags), ctx }
    }

    /// Prepend ``pad_string`` repeatedly until the bin reaches
    /// ``target_length`` codepoints. No-op if already at or above target.
    #[staticmethod]
    #[pyo3(signature = (bin, target_length, pad_string, *, flags=0, ctx=None))]
    pub fn pad_start(bin: String, target_length: i64, pad_string: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringPadStart(bin, target_length, pad_string, flags), ctx }
    }

    /// Append ``pad_string`` repeatedly until the bin reaches
    /// ``target_length`` codepoints. No-op if already at or above target.
    #[staticmethod]
    #[pyo3(signature = (bin, target_length, pad_string, *, flags=0, ctx=None))]
    pub fn pad_end(bin: String, target_length: i64, pad_string: String, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringPadEnd(bin, target_length, pad_string, flags), ctx }
    }

    /// Repeat the bin contents ``count`` times. ``count`` must be non-negative.
    #[staticmethod]
    #[pyo3(signature = (bin, count, *, flags=0, ctx=None))]
    pub fn repeat(bin: String, count: i64, flags: u8, ctx: Option<Vec<CTX>>) -> Self {
        StringOperation { op: OperationType::StringRepeat(bin, count, flags), ctx }
    }

    /// Replace the first match of ``pattern`` (ICU regex) with
    /// ``replacement``. Pass ``StringRegexFlags.GLOBAL`` in ``flags`` to
    /// replace every match.
    ///
    /// ``flags`` carries the regex flags; ``write_flags`` carries the
    /// ``StringWriteFlags`` bitmask (``UPDATE_ONLY`` / ``NO_FAIL``). The
    /// server rejects ``CREATE_ONLY`` here because a regex replace cannot
    /// create a bin.
    #[staticmethod]
    #[pyo3(signature = (bin, pattern, replacement, flags=0, *, write_flags=0, ctx=None))]
    pub fn regex_replace(
        bin: String, pattern: String, replacement: String, flags: u8, write_flags: u8, ctx: Option<Vec<CTX>>,
    ) -> Self {
        StringOperation { op: OperationType::StringRegexReplace(bin, pattern, replacement, flags, write_flags), ctx }
    }

    // -----------------------------------------------------------------
    // Type conversion (TO_STRING, op-type 19 — no payload, no CTX)
    // -----------------------------------------------------------------

    /// Convert a bool, integer, float, string, or blob bin to its string
    /// representation. Returns BIN_TYPE_ERROR for any other bin type, and
    /// OP_NOT_APPLICABLE (subcode ``OPNOT_STRING_UTF8_INVALID``) for a blob
    /// that is not valid UTF-8.
    ///
    /// Note: ``to_string`` does NOT accept a CTX argument — the wire format
    /// is a top-level op with no payload, so there is no place to put a
    /// CTX path. To convert a value nested inside a list or map, extract
    /// the leaf with ``ListOperation`` / ``MapOperation`` first.
    #[staticmethod]
    pub fn to_string(bin: String) -> Self {
        StringOperation { op: OperationType::StringToString(bin), ctx: None }
    }
}
