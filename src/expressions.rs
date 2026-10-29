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

use std::collections::BTreeMap;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};



use crate::cdt::*;
use crate::errors::RustClientError;
use crate::record::{PythonValue, Vector};
use crate::string_ops::StringNumericType;

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ExpressionType (ExpType)
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Expression Data Types for usage in some `FilterExpressions`
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy)]
    pub enum ExpType {
        #[pyo3(name = "NIL")]
        Nil,
        #[pyo3(name = "BOOL")]
        Bool,
        #[pyo3(name = "INT")]
        Int,
        #[pyo3(name = "STRING")]
        String,
        #[pyo3(name = "LIST")]
        List,
        #[pyo3(name = "MAP")]
        Map,
        #[pyo3(name = "BLOB")]
        Blob,
        #[pyo3(name = "FLOAT")]
        Float,
        #[pyo3(name = "GEO")]
        Geo,
        #[pyo3(name = "HLL")]
        HLL,
        #[pyo3(name = "VECTOR")]
        Vector,
    }

    impl From<&ExpType> for aerospike_core::expressions::ExpType {
        fn from(input: &ExpType) -> Self {
            match &input {
                ExpType::Nil => aerospike_core::expressions::ExpType::Nil,
                ExpType::Bool => aerospike_core::expressions::ExpType::Bool,
                ExpType::Int => aerospike_core::expressions::ExpType::Int,
                ExpType::String => aerospike_core::expressions::ExpType::String,
                ExpType::List => aerospike_core::expressions::ExpType::List,
                ExpType::Map => aerospike_core::expressions::ExpType::Map,
                ExpType::Blob => aerospike_core::expressions::ExpType::Blob,
                ExpType::Float => aerospike_core::expressions::ExpType::Float,
                ExpType::Geo => aerospike_core::expressions::ExpType::Geo,
                ExpType::HLL => aerospike_core::expressions::ExpType::Hll,
                ExpType::Vector => aerospike_core::expressions::ExpType::Vector,
            }
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  VectorDistanceMetric
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Metric of :meth:`FilterExpression.vector_distance`.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum VectorDistanceMetric {
        /// Squared Euclidean distance. Smaller is closer.
        #[pyo3(name = "EUCLIDEAN_SQUARED")]
        EuclideanSquared,
        /// Dot product. Larger is more similar.
        #[pyo3(name = "DOT_PRODUCT")]
        DotProduct,
        /// Cosine similarity. Larger is more similar.
        #[pyo3(name = "COSINE_SIMILARITY")]
        CosineSimilarity,
    }

    #[pymethods]
    impl VectorDistanceMetric {
        fn __richcmp__(&self, other: &VectorDistanceMetric, op: pyo3::class::basic::CompareOp) -> PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Err(pyo3::exceptions::PyNotImplementedError::new_err("Only == and != comparisons are supported")),
            }
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    #[pymethods]
    impl ExpType {
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ExpWriteFlags
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Expression write flags for expression operations.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, name = "ExpWriteFlags", module = "_native")]
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ExpWriteFlags {
        /// Default. Allow create or update.
        #[pyo3(name = "DEFAULT")]
        Default = 0,
        /// If bin does not exist, a new bin will be created.
        /// If bin exists, the operation will be denied.
        #[pyo3(name = "CREATE_ONLY")]
        CreateOnly = 1,
        /// If bin exists, the bin will be overwritten.
        /// If bin does not exist, the operation will be denied.
        #[pyo3(name = "UPDATE_ONLY")]
        UpdateOnly = 2,
        /// If expression results in nil value, then delete the bin.
        #[pyo3(name = "ALLOW_DELETE")]
        AllowDelete = 4,
        /// Do not raise error if operation is denied.
        #[pyo3(name = "POLICY_NO_FAIL")]
        PolicyNoFail = 8,
        /// Ignore failures caused by the expression resolving to unknown or a non-bin type.
        #[pyo3(name = "EVAL_NO_FAIL")]
        EvalNoFail = 16,
    }

    impl From<ExpWriteFlags> for u8 {
        fn from(flags: ExpWriteFlags) -> Self {
            flags as u8
        }
    }

    /// Extract flags as i64 from ExpWriteFlags or int (bitmask).
    /// Used by ExpOperation.write and ExpWriteFlags' own __or__ / __ror__ dunders.
    pub(crate) fn exp_write_flags_from_py(ob: &Bound<'_, PyAny>) -> PyResult<i64> {
        if let Ok(f) = ob.extract::<ExpWriteFlags>() {
            return Ok(u8::from(f) as i64);
        }
        if let Ok(i) = ob.extract::<i64>() {
            return Ok(i);
        }
        Err(pyo3::exceptions::PyValueError::new_err(
            "flags must be ExpWriteFlags or int",
        ))
    }

    #[pymethods]
    impl ExpWriteFlags {
        /// Combine flags with bitwise OR. Returns the wire byte as int so combined
        /// values can be passed wherever a single flag is accepted.
        fn __or__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_write_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Right-hand bitwise OR (e.g. `int | ExpWriteFlags.X`).
        fn __ror__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_write_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Bitwise AND.
        fn __and__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_write_flags_from_py(other)?;
            Ok(a & b)
        }

        /// Right-hand bitwise AND.
        fn __rand__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_write_flags_from_py(other)?;
            Ok(b & a)
        }

        /// Bitwise XOR.
        fn __xor__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_write_flags_from_py(other)?;
            Ok(a ^ b)
        }

        /// Right-hand bitwise XOR.
        fn __rxor__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_write_flags_from_py(other)?;
            Ok(b ^ a)
        }

        /// Bitwise NOT (masked to u8 flag width, returned as int).
        fn __invert__(&self) -> i64 {
            (!u8::from(*self)) as i64
        }

        /// Equality / ordering against another ``ExpWriteFlags`` *or* an
        /// ``int`` bitmask. This honors the ``IntEnum`` runtime contract
        /// promised by the generated stubs.
        fn __richcmp__(
            &self,
            other: &Bound<'_, PyAny>,
            op: pyo3::class::basic::CompareOp,
        ) -> pyo3::PyResult<bool> {
            let a = u8::from(*self) as i64;
            let b = match exp_write_flags_from_py(other) {
                Ok(v) => v,
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

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  ExpReadFlags
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Expression read flags for expression operations.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, name = "ExpReadFlags", module = "_native")]
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ExpReadFlags {
        /// Default.
        #[pyo3(name = "DEFAULT")]
        Default = 0,
        /// Ignore failures caused by the expression resolving to unknown or a non-bin type.
        #[pyo3(name = "EVAL_NO_FAIL")]
        EvalNoFail = 16,
    }

    impl From<ExpReadFlags> for u8 {
        fn from(flags: ExpReadFlags) -> Self {
            flags as u8
        }
    }

    /// Extract flags as i64 from ExpReadFlags or int (bitmask).
    /// Used by ExpOperation.read and ExpReadFlags' own __or__ / __ror__ dunders.
    pub(crate) fn exp_read_flags_from_py(ob: &Bound<'_, PyAny>) -> PyResult<i64> {
        if let Ok(f) = ob.extract::<ExpReadFlags>() {
            return Ok(u8::from(f) as i64);
        }
        if let Ok(i) = ob.extract::<i64>() {
            return Ok(i);
        }
        Err(pyo3::exceptions::PyValueError::new_err(
            "flags must be ExpReadFlags or int",
        ))
    }

    #[pymethods]
    impl ExpReadFlags {
        /// Combine flags with bitwise OR. Returns the wire byte as int so combined
        /// values can be passed wherever a single flag is accepted.
        fn __or__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_read_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Right-hand bitwise OR (e.g. `int | ExpReadFlags.X`).
        fn __ror__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_read_flags_from_py(other)?;
            Ok(a | b)
        }

        /// Bitwise AND.
        fn __and__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_read_flags_from_py(other)?;
            Ok(a & b)
        }

        /// Right-hand bitwise AND.
        fn __rand__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_read_flags_from_py(other)?;
            Ok(b & a)
        }

        /// Bitwise XOR.
        fn __xor__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_read_flags_from_py(other)?;
            Ok(a ^ b)
        }

        /// Right-hand bitwise XOR.
        fn __rxor__(&self, other: &Bound<'_, PyAny>) -> PyResult<i64> {
            let a = u8::from(*self) as i64;
            let b = exp_read_flags_from_py(other)?;
            Ok(b ^ a)
        }

        /// Bitwise NOT (masked to u8 flag width, returned as int).
        fn __invert__(&self) -> i64 {
            (!u8::from(*self)) as i64
        }

        /// Equality / ordering against another ``ExpReadFlags`` *or* an
        /// ``int`` bitmask. This honors the ``IntEnum`` runtime contract
        /// promised by the generated stubs.
        fn __richcmp__(
            &self,
            other: &Bound<'_, PyAny>,
            op: pyo3::class::basic::CompareOp,
        ) -> pyo3::PyResult<bool> {
            let a = u8::from(*self) as i64;
            let b = match exp_read_flags_from_py(other) {
                Ok(v) => v,
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

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  RegexFlag
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// POSIX regex bit flags for ``FilterExpression.regex_compare``.
    ///
    /// Bit values match the Aerospike server wire protocol (POSIX ``regex.h``
    /// on glibc):
    ///
    /// - ``NONE = 0`` — use regex defaults.
    /// - ``EXTENDED = 1`` — POSIX Extended Regular Expression syntax.
    /// - ``ICASE = 2`` — case-insensitive matching.
    /// - ``NOSUB = 4`` — do not report position of matches.
    /// - ``NEWLINE = 8`` — match-any-character operators don't match newline.
    ///
    /// Combine with bitwise OR, e.g. ``RegexFlag.ICASE | RegexFlag.NEWLINE``.
    /// The ``regex_compare`` ``flags`` parameter accepts ``int`` or any
    /// ``RegexFlag`` constant (or combination). These are not interchangeable
    /// with ``StringRegexFlags``, which apply to the ICU-syntax string
    /// expression ``string_regex_compare_with_flags``.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(name = "RegexFlag", module = "_native")]
    pub struct RegexFlag;

    #[gen_stub_pymethods]
    #[pymethods]
    impl RegexFlag {
        /// Use regex defaults.
        #[classattr]
        const NONE: i64 = 0;
        /// Use POSIX Extended Regular Expression syntax when interpreting regex.
        #[classattr]
        const EXTENDED: i64 = 1;
        /// Do not differentiate case.
        #[classattr]
        const ICASE: i64 = 2;
        /// Do not report position of matches.
        #[classattr]
        const NOSUB: i64 = 4;
        /// Match-any-character operators don't match a newline.
        #[classattr]
        const NEWLINE: i64 = 8;
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Filter Expression
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Filter expression, which can be applied to most commands, to control which records are
    /// affected by the command.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, 
        name = "FilterExpression",
        module = "_native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone, Debug)]
    pub struct FilterExpression {
        pub(crate) _as: aerospike_core::expressions::Expression,
    }

    impl PartialEq for FilterExpression {
        fn eq(&self, other: &Self) -> bool {
            // For now, we'll use a simple approach - compare the debug representation
            // This is not perfect but will work for testing purposes
            format!("{:?}", self._as) == format!("{:?}", other._as)
        }
    }

    impl Eq for FilterExpression {}

    impl std::hash::Hash for FilterExpression {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            // Use the debug representation for hashing
            format!("{:?}", self._as).hash(state);
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl FilterExpression {
        #[staticmethod]
        /// Create a record key expression of specified type.
        pub fn key(exp_type: ExpType) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::key((&exp_type).into()),
            }
        }

        #[staticmethod]
        /// Create function that returns if the primary key is stored in the record meta data
        /// as a boolean expression. This would occur when `send_key` is true on record write.
        pub fn key_exists() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::key_exists(),
            }
        }

        #[staticmethod]
        /// Create 64 bit int bin expression.
        pub fn int_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_bin(name),
            }
        }

        #[staticmethod]
        /// Create string bin expression.
        pub fn string_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::string_bin(name),
            }
        }

        #[staticmethod]
        /// Create blob bin expression.
        pub fn blob_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::blob_bin(name),
            }
        }

        #[staticmethod]
        /// Create boolean bin expression.
        pub fn bool_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::bool_bin(name),
            }
        }

        #[staticmethod]
        /// Create 64 bit float bin expression.
        pub fn float_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::float_bin(name),
            }
        }

        #[staticmethod]
        /// Create geo bin expression.
        pub fn geo_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::geo_bin(name),
            }
        }

        #[staticmethod]
        /// Create list bin expression.
        pub fn list_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::list_bin(name),
            }
        }

        #[staticmethod]
        /// Create map bin expression.
        pub fn map_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::map_bin(name),
            }
        }

        #[staticmethod]
        /// Create a HLL bin expression
        pub fn hll_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::hll_bin(name),
            }
        }

        #[staticmethod]
        /// Create a vector bin expression, the stored side of :meth:`vector_distance`.
        pub fn vector_bin(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::vector_bin(name),
            }
        }

        #[staticmethod]
        /// Create an expression that returns, as a float, the distance between
        /// ``query`` and a stored vector under ``metric``.
        ///
        /// ``bin`` is typically :meth:`vector_bin`. A stored value that is not a
        /// vector, or whose element type or dimension count differs from
        /// ``query``, evaluates to unknown: the record does not match as a filter,
        /// and an ``ExpOperation.read`` with ``ExpReadFlags.EVAL_NO_FAIL`` returns
        /// no value for it. Requires every node to run Aerospike Server 8.2.1 or
        /// later.
        ///
        /// Example::
        ///
        ///     close = FilterExpression.lt(
        ///         FilterExpression.vector_distance(
        ///             VectorDistanceMetric.EUCLIDEAN_SQUARED,
        ///             Vector([0.1, 0.9, 0.3]),
        ///             FilterExpression.vector_bin("embedding"),
        ///         ),
        ///         FilterExpression.float_val(0.25),
        ///     )
        pub fn vector_distance(metric: VectorDistanceMetric, query: PyRef<'_, Vector>, bin: FilterExpression) -> Self {
            use aerospike_core::expressions::vector;
            let build = match metric {
                VectorDistanceMetric::EuclideanSquared => vector::euclidean_squared_distance,
                VectorDistanceMetric::DotProduct => vector::dot_product,
                VectorDistanceMetric::CosineSimilarity => vector::cosine_similarity,
            };
            FilterExpression {
                _as: build(&query.v, bin._as),
            }
        }

        #[staticmethod]
        /// Create function that returns if bin of specified name exists.
        pub fn bin_exists(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::ne(
                    aerospike_core::expressions::bin_type(name),
                    aerospike_core::expressions::int_val(0_i64),
                ),
            }
        }

        #[staticmethod]
        /// Create function that returns bin's integer particle type.
        pub fn bin_type(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::bin_type(name),
            }
        }

        #[staticmethod]
        /// Create function that returns record set name string.
        pub fn set_name() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::set_name(),
            }
        }

        #[staticmethod]
        /// Create expression that returns the record size. Usually evaluates quickly because
        /// record metadata is cached in memory. Requires server version 7.0+.
        pub fn record_size() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::record_size(),
            }
        }

        #[staticmethod]
        /// Create function that returns record last update time expressed as 64 bit integer
        /// nanoseconds since 1970-01-01 epoch.
        pub fn last_update() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::last_update(),
            }
        }

        #[staticmethod]
        /// Create expression that returns milliseconds since the record was last updated.
        /// This expression usually evaluates quickly because record meta data is cached in memory.
        pub fn since_update() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::since_update(),
            }
        }

        #[staticmethod]
        /// Create function that returns record expiration time expressed as 64 bit integer
        /// nanoseconds since 1970-01-01 epoch.
        pub fn void_time() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::void_time(),
            }
        }

        #[staticmethod]
        /// Create function that returns record expiration time (time to live) in integer seconds.
        pub fn ttl() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::ttl(),
            }
        }

        #[staticmethod]
        /// Create expression that returns if record has been deleted and is still in tombstone state.
        /// This expression usually evaluates quickly because record meta data is cached in memory.
        pub fn is_tombstone() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::is_tombstone(),
            }
        }

        #[staticmethod]
        /// Create function that returns record digest modulo as integer.
        pub fn digest_modulo(modulo: i64) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::digest_modulo(modulo),
            }
        }

        #[staticmethod]
        /// Create a POSIX regular expression match on a string bin or string value expression.
        ///
        /// Uses ``RegexFlag`` bits and works on all supported servers. This is a
        /// different server operation from the ICU-syntax ``string_regex_compare``
        /// (server >= 8.2.0), not an older spelling of it.
        pub fn regex_compare(regex: String, flags: i64, bin: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::regex_compare(
                    regex,
                    aerospike_core::expressions::RegexFlags::from_bits(flags),
                    bin._as,
                ),
            }
        }

        #[staticmethod]
        /// Create compare geospatial operation.
        pub fn geo_compare(left: FilterExpression, right: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::geo_compare(left._as, right._as),
            }
        }

        #[staticmethod]
        /// Build a value expression from a Python value, dispatching by type.
        ///
        /// Single-entry alternative to the typed accessors (``bool_val``,
        ///  ``int_val``, ``float_val``, ``string_val``, ``blob_val``,
        ///  ``list_val``, ``map_val``, ``geo_val``, ``nil``). The Python type of
        /// *value* selects which underlying constructor is invoked; ``None``
        /// maps to :meth:`nil`. ``GeoJSON`` strings and HLL bytes should
        /// continue to use :meth:`geo_val` / typed accessors explicitly when
        /// the literal form is ambiguous.
        ///
        /// Raises ``TypeError`` for values that don't correspond to a
        /// supported variant (e.g. ``SpecialValue`` sentinels).
        pub fn val(value: PythonValue) -> PyResult<Self> {
            Ok(match value {
                PythonValue::Nil       => Self::nil(),
                PythonValue::Bool(b)   => Self::bool_val(b),
                PythonValue::Int(i)    => Self::int_val(i),
                PythonValue::Float(f)  => Self::float_val(f.into_inner()),
                PythonValue::String(s) => Self::string_val(s),
                PythonValue::Blob(b)   => Self::blob_val(b),
                PythonValue::List(l)   => Self::list_val(l),
                v @ (PythonValue::HashMap(_)
                | PythonValue::OrderedMap(_)
                | PythonValue::SortedMap(_)) => Self::map_val(v),
                PythonValue::GeoJSON(s) => Self::geo_val(s),
                other => {
                    return Err(PyTypeError::new_err(format!(
                        "Exp.val: unsupported value type for a literal value expression: {:?}",
                        other,
                    )));
                }
            })
        }

        #[staticmethod]
        /// Creates 64 bit integer value
        pub fn int_val(val: i64) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_val(val),
            }
        }

        #[staticmethod]
        /// Creates a Boolean value
        pub fn bool_val(val: bool) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::bool_val(val),
            }
        }

        #[staticmethod]
        /// Creates String bin value
        pub fn string_val(val: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::string_val(val),
            }
        }

        #[staticmethod]
        /// Creates 64 bit float bin value
        pub fn float_val(val: f64) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::float_val(val),
            }
        }

        #[staticmethod]
        /// Creates Blob bin value
        pub fn blob_val(val: Vec<u8>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::blob_val(val),
            }
        }

        #[staticmethod]
        /// Create List bin value.
        pub fn list_val(val: Vec<PythonValue>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::list_val(
                    val.into_iter()
                        .map(aerospike_core::Value::from)
                        .collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create Map bin value.
        /// Converts HashMap to BTreeMap (OrderedMap) for exact byte-level matching.
        /// map_val accepts both HashMap and BTreeMap (OrderedMap) via the MapLike trait.
        /// We use BTreeMap to ensure deterministic key ordering for serialization matching.
        pub fn map_val(val: PythonValue) -> Self {
            match val {
                // Declared key-ordered: pack with the K-ordered flag, which is
                // what a bin written from a SortedMap carries. Whole-map
                // comparison currently requires both operands to be K-ordered;
                // the unordered case is blocked on SERVER-94.
                PythonValue::SortedMap(pairs) => {
                    let mut btree: BTreeMap<aerospike_core::Value, aerospike_core::Value> =
                        BTreeMap::new();
                    for (k, v) in pairs {
                        btree.insert(
                            aerospike_core::Value::from(k),
                            aerospike_core::Value::from(v),
                        );
                    }
                    FilterExpression {
                        _as: aerospike_core::expressions::map_val(btree),
                    }
                }
                PythonValue::HashMap(h) => {
                    // Convert to BTreeMap for deterministic key ordering. Rust HashMap iteration
                    // order is non-deterministic, so we sort keys to ensure exact byte-level matching
                    // with the server's stored format (which uses sorted keys for KEY_ORDERED maps).
                    let mut btree_map: BTreeMap<aerospike_core::Value, aerospike_core::Value> = BTreeMap::new();
                    for (k, v) in h {
                        btree_map.insert(aerospike_core::Value::from(k), aerospike_core::Value::from(v));
                    }

                    // BTreeMap implements MapLike, so we can pass it directly
                    FilterExpression {
                        _as: aerospike_core::expressions::map_val(btree_map),
                    }
                }
                _ => panic!("map_val requires a map value (HashMap)"),
            }
        }

        #[staticmethod]
        /// Create geospatial json string value.
        pub fn geo_val(val: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::geo_val(val),
            }
        }

        #[staticmethod]
        /// Create a Nil PHPValue
        pub fn nil() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::nil(),
            }
        }

        #[staticmethod]
        /// Create an infinity value for expression and CDT range boundaries.
        pub fn infinity() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::infinity(),
            }
        }

        #[staticmethod]
        /// Create a wildcard value for expression and CDT value matching.
        pub fn wildcard() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::wildcard(),
            }
        }

        #[staticmethod]
        #[pyo3(name = "not_")]
        /// Create "not" operator expression.
        pub fn not(exp: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::not(exp._as),
            }
        }

        #[staticmethod]
        #[pyo3(name = "and_")]
        /// Create "and" (&&) operator that applies to a variable number of expressions.
        /// // (a > 5 || a == 0) && b < 3
        pub fn and(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::and(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        #[pyo3(name = "or_")]
        /// Create "or" (||) operator that applies to a variable number of expressions.
        pub fn or(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::or(exps.into_iter().map(|exp| exp._as).collect()),
            }
        }

        #[staticmethod]
        /// Create an expression that is true when exactly one of the expressions
        /// is true. Same operator as ``exclusive``; unlike a bitwise ``^``, three
        /// true operands evaluate to false. The server requires at least two.
        pub fn xor(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::xor(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create an expression that is true when exactly one of the expressions
        /// is true. The server requires at least two.
        pub fn exclusive(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exclusive(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create equal (==) expression.
        pub fn eq(left: FilterExpression, right: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::eq(left._as, right._as),
            }
        }

        #[staticmethod]
        /// Create not equal (!=) expression
        pub fn ne(left: FilterExpression, right: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::ne(left._as, right._as),
            }
        }

        #[staticmethod]
        /// Create greater than (>) operation.
        pub fn gt(left: FilterExpression, right: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::gt(left._as, right._as),
            }
        }

        #[staticmethod]
        /// Create greater than or equal (>=) operation.
        pub fn ge(left: FilterExpression, right: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::ge(left._as, right._as),
            }
        }

        #[staticmethod]
        /// Create less than (<) operation.
        pub fn lt(left: FilterExpression, right: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::lt(left._as, right._as),
            }
        }

        #[staticmethod]
        /// Create less than or equals (<=) operation.
        pub fn le(left: FilterExpression, right: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::le(left._as, right._as),
            }
        }

        #[staticmethod]
        /// Create "add" (+) operator that applies to a variable number of expressions.
        /// Return sum of all `FilterExpressions` given. All arguments must resolve to the same type (integer or float).
        /// Requires server version 5.6.0+.
        pub fn num_add(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_add(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create "subtract" (-) operator that applies to a variable number of expressions.
        /// If only one `FilterExpressions` is provided, return the negation of that argument.
        /// Otherwise, return the sum of the 2nd to Nth `FilterExpressions` subtracted from the 1st
        /// `FilterExpressions`. All `FilterExpressions` must resolve to the same type (integer or float).
        /// Requires server version 5.6.0+.
        pub fn num_sub(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_sub(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create "multiply" (*) operator that applies to a variable number of expressions.
        /// Return the product of all `FilterExpressions`. If only one `FilterExpressions` is supplied, return
        /// that `FilterExpressions`. All `FilterExpressions` must resolve to the same type (integer or float).
        /// Requires server version 5.6.0+.
        pub fn num_mul(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_mul(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create "divide" (/) operator that applies to a variable number of expressions.
        /// If there is only one `FilterExpressions`, returns the reciprocal for that `FilterExpressions`.
        /// Otherwise, return the first `FilterExpressions` divided by the product of the rest.
        /// All `FilterExpressions` must resolve to the same type (integer or float).
        /// Requires server version 5.6.0+.
        pub fn num_div(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_div(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create "power" operator that raises a "base" to the "exponent" power.
        /// All arguments must resolve to floats.
        /// Requires server version 5.6.0+.
        pub fn num_pow(base: FilterExpression, exponent: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_pow(base._as, exponent._as),
            }
        }

        #[staticmethod]
        /// Create "log" operator for logarithm of "num" with base "base".
        /// All arguments must resolve to floats.
        /// Requires server version 5.6.0+.
        pub fn num_log(num: FilterExpression, base: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_log(num._as, base._as),
            }
        }

        #[staticmethod]
        /// Create "modulo" (%) operator that determines the remainder of "numerator"
        /// divided by "denominator". All arguments must resolve to integers.
        /// Requires server version 5.6.0+.
        pub fn num_mod(numerator: FilterExpression, denominator: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_mod(numerator._as, denominator._as),
            }
        }

        #[staticmethod]
        /// Create operator that returns absolute value of a number.
        /// All arguments must resolve to integer or float.
        /// Requires server version 5.6.0+.
        pub fn num_abs(value: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_abs(value._as),
            }
        }

        #[staticmethod]
        /// Create expression that rounds a floating point number down to the closest integer value.
        /// The return type is float.
        // Requires server version 5.6.0+.
        pub fn num_floor(num: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_floor(num._as),
            }
        }

        #[staticmethod]
        /// Create expression that rounds a floating point number up to the closest integer value.
        /// The return type is float.
        /// Requires server version 5.6.0+.
        pub fn num_ceil(num: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::num_ceil(num._as),
            }
        }

        #[staticmethod]
        /// Create expression that converts an integer to a float.
        /// Requires server version 5.6.0+.
        pub fn to_int(num: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::to_int(num._as),
            }
        }

        #[staticmethod]
        /// Create expression that converts a float to an integer.
        /// Requires server version 5.6.0+.
        pub fn to_float(num: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::to_float(num._as),
            }
        }

        #[staticmethod]
        /// Create integer "and" (&) operator that is applied to two or more integers.
        /// All arguments must resolve to integers.
        /// Requires server version 5.6.0+.
        pub fn int_and(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_and(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create integer "or" (|) operator that is applied to two or more integers.
        /// All arguments must resolve to integers.
        /// Requires server version 5.6.0+.
        pub fn int_or(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_or(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create integer "xor" (^) operator that is applied to two or more integers.
        /// All arguments must resolve to integers.
        /// Requires server version 5.6.0+.
        pub fn int_xor(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_xor(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create integer "not" (~) operator.
        /// Requires server version 5.6.0+.
        pub fn int_not(exp: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_not(exp._as),
            }
        }

        #[staticmethod]
        /// Create integer "left shift" (<<) operator.
        /// Requires server version 5.6.0+.
        pub fn int_lshift(value: FilterExpression, shift: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_lshift(value._as, shift._as),
            }
        }

        #[staticmethod]
        /// Create integer "logical right shift" (>>>) operator.
        /// Requires server version 5.6.0+.
        pub fn int_rshift(value: FilterExpression, shift: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_rshift(value._as, shift._as),
            }
        }

        #[staticmethod]
        /// Create integer "arithmetic right shift" (>>) operator.
        /// The sign bit is preserved and not shifted.
        /// Requires server version 5.6.0+.
        pub fn int_arshift(value: FilterExpression, shift: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_arshift(value._as, shift._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns count of integer bits that are set to 1.
        /// Requires server version 5.6.0+
        pub fn int_count(exp: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_count(exp._as),
            }
        }

        #[staticmethod]
        /// Create expression that scans integer bits from left (most significant bit) to
        /// right (least significant bit), looking for a search bit value. When the
        /// search value is found, the index of that bit (where the most significant bit is
        /// index 0) is returned. If "search" is true, the scan will search for the bit
        /// value 1. If "search" is false it will search for bit value 0.
        /// Requires server version 5.6.0+.
        pub fn int_lscan(value: FilterExpression, search: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_lscan(value._as, search._as),
            }
        }

        #[staticmethod]
        /// Create expression that scans integer bits from right (least significant bit) to
        /// left (most significant bit), looking for a search bit value. When the
        /// search value is found, the index of that bit (where the most significant bit is
        /// index 0) is returned. If "search" is true, the scan will search for the bit
        /// value 1. If "search" is false it will search for bit value 0.
        /// Requires server version 5.6.0+.
        pub fn int_rscan(value: FilterExpression, search: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::int_rscan(value._as, search._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns the minimum value in a variable number of expressions.
        /// All arguments must be the same type (integer or float).
        /// Requires server version 5.6.0+.
        pub fn min(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::min(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that returns the maximum value in a variable number of expressions.
        /// All arguments must be the same type (integer or float).
        /// Requires server version 5.6.0+.
        pub fn max(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::max(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        //--------------------------------------------------
        // Variables
        //--------------------------------------------------

        #[staticmethod]
        /// Conditionally select an expression from a variable number of expression pairs
        /// followed by default expression action.
        /// Requires server version 5.6.0+.
        /// ```
        /// // Args Format: bool exp1, action exp1, bool exp2, action exp2, ..., action-default
        /// // Apply operator based on type.
        pub fn cond(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::cond(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        /// Define variables and expressions in scope.
        /// Requires server version 5.6.0+.
        /// ```
        /// // 5 < a < 10
        pub fn exp_let(exps: Vec<FilterExpression>) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_let(
                    exps.into_iter().map(|exp| exp._as).collect(),
                ),
            }
        }

        #[staticmethod]
        #[pyo3(name = "def_")]
        /// Assign variable to an expression that can be accessed later.
        /// Requires server version 5.6.0+.
        /// ```
        /// // 5 < a < 10
        pub fn def(name: String, value: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::def(name, value._as),
            }
        }

        #[staticmethod]
        /// Retrieve expression value from a variable.
        /// Requires server version 5.6.0+.
        pub fn var(name: String) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::var(name),
            }
        }

        fn __richcmp__(&self, other: &FilterExpression, op: pyo3::class::basic::CompareOp) -> pyo3::PyResult<bool> {
            match op {
                pyo3::class::basic::CompareOp::Eq => Ok(self == other),
                pyo3::class::basic::CompareOp::Ne => Ok(self != other),
                _ => Ok(false),
            }
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }

        /// Return the debug representation of the inner expression (used for equality).
        /// Exposed for inspection; same string used by __eq__.
        pub fn _debug_inner(&self) -> String {
            format!("{:?}", self._as)
        }

        /// Encode the expression to a base64 string.
        pub fn base64(&self) -> PyResult<String> {
            self._as
                .base64()
                .map_err(|e| PyErr::from(RustClientError(e)))
        }

        #[staticmethod]
        /// Create an expression from a base64-encoded expression string.
        pub fn from_base64(b64: &str) -> PyResult<FilterExpression> {
            aerospike_core::expressions::from_base64(b64)
                .map(|expr| FilterExpression { _as: expr })
                .map_err(|e| PyErr::from(RustClientError(e)))
        }

        #[staticmethod]
        /// Build a filter expression whose wire form is ``[128, "<ael>"]`` (MessagePack), so the
        /// server (8.2.0+) parses and compiles the Aerospike Expression Language string.
        pub fn from_server_compiled_ael(ael: &str) -> PyResult<FilterExpression> {
            aerospike_core::expressions::pack_ael_server_filter(ael)
                .map(|expr| FilterExpression { _as: expr })
                .map_err(|e| PyErr::from(RustClientError(e)))
        }

        #[staticmethod]
        /// Create unknown value. Used to intentionally fail an expression.
        /// The failure can be ignored with `ExpWriteFlags` `EVAL_NO_FAIL`
        /// or `ExpReadFlags` `EVAL_NO_FAIL`.
        /// Requires server version 5.6.0+.
        pub fn unknown() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::unknown(),
            }
        }

        //--------------------------------------------------
        // List CDT Expressions
        //--------------------------------------------------

        #[staticmethod]
        /// Create expression that returns list size.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_size(bin: FilterExpression, ctx: Vec<CTX>) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: lists::size(bin._as, &ctx_vec),
            }
        }

        #[staticmethod]
        /// Returns STRING — the string items of the list concatenated. The
        /// list must hold only strings. The inverse of ``string_split``.
        /// Requires server 8.2.0+.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_join(bin: FilterExpression, ctx: Vec<CTX>) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: lists::join(bin._as, &ctx_vec),
            }
        }

        #[staticmethod]
        /// Returns STRING — the string items of the list concatenated with
        /// `separator` between consecutive items. The list must hold only
        /// strings. The inverse of ``string_split_by_separator``.
        /// Requires server 8.2.0+.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_join_by_separator(
            separator: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: lists::join_by_separator(separator._as, bin._as, &ctx_vec),
            }
        }

        #[staticmethod]
        /// Create expression that selects list item identified by index and returns
        /// selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_index(
            return_type: ListReturnType,
            value_type: ExpType,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_index(
                    core_return_type,
                    (&value_type).into(),
                    index._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects list item identified by rank and returns
        /// selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_rank(
            return_type: ListReturnType,
            value_type: ExpType,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_rank(
                    core_return_type,
                    (&value_type).into(),
                    rank._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects list items identified by value and returns selected data
        /// specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_value(
            return_type: ListReturnType,
            value: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_value(
                    core_return_type,
                    value._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects list items identified by value range (valueBegin inclusive, valueEnd exclusive)
        /// and returns selected data specified by returnType.
        /// If valueBegin is None, the range is less than valueEnd. If valueEnd is None, the range is greater than equal to valueBegin.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_value_range(
            return_type: ListReturnType,
            value_begin: Option<FilterExpression>,
            value_end: Option<FilterExpression>,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_value_range(
                    core_return_type,
                    value_begin.as_ref().map(|v| v._as.clone()),
                    value_end.as_ref().map(|v| v._as.clone()),
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects list items identified by values and returns selected data
        /// specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_value_list(
            return_type: ListReturnType,
            values: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_value_list(
                    core_return_type,
                    values._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects list items starting at specified index to the end of list
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_index_range(
            return_type: ListReturnType,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_index_range(
                    core_return_type,
                    index._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects "count" list items starting at specified index
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_index_range_count(
            return_type: ListReturnType,
            index: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_index_range_count(
                    core_return_type,
                    index._as,
                    count._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects list items starting at specified rank to the last ranked item
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_rank_range(
            return_type: ListReturnType,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_rank_range(
                    core_return_type,
                    rank._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects "count" list items starting at specified rank and returns
        /// selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_rank_range_count(
            return_type: ListReturnType,
            rank: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_rank_range_count(
                    core_return_type,
                    rank._as,
                    count._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects list items nearest to value and greater by relative rank
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_value_relative_rank_range(
            return_type: ListReturnType,
            value: FilterExpression,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_value_relative_rank_range(
                    core_return_type,
                    value._as,
                    rank._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects list items nearest to value and greater by relative rank with a count limit
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn list_get_by_value_relative_rank_range_count(
            return_type: ListReturnType,
            value: FilterExpression,
            rank: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: lists::get_by_value_relative_rank_range_count(
                    core_return_type,
                    value._as,
                    rank._as,
                    count._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        //--------------------------------------------------
        // List CDT Write Expressions
        //--------------------------------------------------

        #[staticmethod]
        /// Create expression that appends value to end of list.
        pub fn list_append(
            policy: ListPolicy,
            value: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::append(policy._as, value._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that appends list items to end of list.
        pub fn list_append_items(
            policy: ListPolicy,
            list: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::append_items(policy._as, list._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that inserts value to specified index of list.
        pub fn list_insert(
            policy: ListPolicy,
            index: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::insert(policy._as, index._as, value._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that inserts each input list item starting at specified index of list.
        pub fn list_insert_items(
            policy: ListPolicy,
            index: FilterExpression,
            list: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::insert_items(policy._as, index._as, list._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that increments list[index] by value.
        pub fn list_increment(
            policy: ListPolicy,
            index: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::increment(policy._as, index._as, value._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that sets item value at specified index in list.
        pub fn list_set(
            policy: ListPolicy,
            index: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::set(policy._as, index._as, value._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes all items in list.
        pub fn list_clear(bin: FilterExpression, ctx: Vec<CTX>) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::clear(bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that sorts list according to sort_flags.
        pub fn list_sort(
            sort_flags: ListSortFlags,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            let core_flags: aerospike_core::operations::lists::ListSortFlags = (&sort_flags).into();
            FilterExpression {
                _as: lists::sort(core_flags, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        //--------------------------------------------------
        // List CDT Remove Expressions
        //--------------------------------------------------

        #[staticmethod]
        /// Create expression that removes list items identified by value.
        pub fn list_remove_by_value(
            return_type: ListReturnType,
            value: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_value(return_type.into(), value._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes list items identified by values.
        pub fn list_remove_by_value_list(
            return_type: ListReturnType,
            values: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_value_list(return_type.into(), values._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes list items identified by value range
        /// (value_begin inclusive, value_end exclusive).
        pub fn list_remove_by_value_range(
            return_type: ListReturnType,
            value_begin: Option<FilterExpression>,
            value_end: Option<FilterExpression>,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_value_range(
                    return_type.into(),
                    value_begin.map(|e| e._as),
                    value_end.map(|e| e._as),
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes list items nearest to value and greater by relative rank.
        pub fn list_remove_by_value_relative_rank_range(
            return_type: ListReturnType,
            value: FilterExpression,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_value_relative_rank_range(
                    return_type.into(),
                    value._as,
                    rank._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes list items nearest to value and greater by relative rank
        /// with a count limit.
        pub fn list_remove_by_value_relative_rank_range_count(
            return_type: ListReturnType,
            value: FilterExpression,
            rank: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_value_relative_rank_range_count(
                    return_type.into(),
                    value._as,
                    rank._as,
                    count._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes list item identified by index.
        pub fn list_remove_by_index(
            return_type: ListReturnType,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_index(return_type.into(), index._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes list items starting at specified index to the end of list.
        pub fn list_remove_by_index_range(
            return_type: ListReturnType,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_index_range(return_type.into(), index._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes "count" list items starting at specified index.
        pub fn list_remove_by_index_range_count(
            return_type: ListReturnType,
            index: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_index_range_count(
                    return_type.into(),
                    index._as,
                    count._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes list item identified by rank.
        pub fn list_remove_by_rank(
            return_type: ListReturnType,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_rank(return_type.into(), rank._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes list items starting at specified rank to the last ranked item.
        pub fn list_remove_by_rank_range(
            return_type: ListReturnType,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_rank_range(return_type.into(), rank._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes "count" list items starting at specified rank.
        pub fn list_remove_by_rank_range_count(
            return_type: ListReturnType,
            rank: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::lists;
            FilterExpression {
                _as: lists::remove_by_rank_range_count(
                    return_type.into(),
                    rank._as,
                    count._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        //--------------------------------------------------
        // Map CDT Expressions
        //--------------------------------------------------

        #[staticmethod]
        /// Create expression that returns map size.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_size(bin: FilterExpression, ctx: Vec<CTX>) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: maps::size(bin._as, &ctx_vec),
            }
        }

        #[staticmethod]
        /// Create expression that selects map item identified by key and returns selected data
        /// specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_key(
            return_type: MapReturnType,
            value_type: ExpType,
            key: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_key(
                    core_return_type,
                    (&value_type).into(),
                    key._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map item identified by rank and returns selected data
        /// specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_rank(
            return_type: MapReturnType,
            value_type: ExpType,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_rank(
                    core_return_type,
                    (&value_type).into(),
                    rank._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map item identified by index and returns selected data
        /// specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_index(
            return_type: MapReturnType,
            value_type: ExpType,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_index(
                    core_return_type,
                    (&value_type).into(),
                    index._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items identified by value and returns selected data
        /// specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_value(
            return_type: MapReturnType,
            value: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_value(
                    core_return_type,
                    value._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items identified by value range (valueBegin inclusive, valueEnd exclusive)
        /// and returns selected data specified by returnType.
        /// If valueBegin is None, the range is less than valueEnd. If valueEnd is None, the range is greater than equal to valueBegin.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_value_range(
            return_type: MapReturnType,
            value_begin: Option<FilterExpression>,
            value_end: Option<FilterExpression>,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_value_range(
                    core_return_type,
                    value_begin.as_ref().map(|v| v._as.clone()),
                    value_end.as_ref().map(|v| v._as.clone()),
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items identified by values and returns selected data
        /// specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_value_list(
            return_type: MapReturnType,
            values: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_value_list(
                    core_return_type,
                    values._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items identified by key range (keyBegin inclusive, keyEnd exclusive)
        /// and returns selected data specified by returnType.
        /// If keyBegin is None, the range is less than keyEnd. If keyEnd is None, the range is greater than equal to keyBegin.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_key_range(
            return_type: MapReturnType,
            key_begin: Option<FilterExpression>,
            key_end: Option<FilterExpression>,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_key_range(
                    core_return_type,
                    key_begin.as_ref().map(|v| v._as.clone()),
                    key_end.as_ref().map(|v| v._as.clone()),
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items identified by keys and returns selected data
        /// specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_key_list(
            return_type: MapReturnType,
            keys: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_key_list(
                    core_return_type,
                    keys._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items nearest to key and greater by index
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_key_relative_index_range(
            return_type: MapReturnType,
            key: FilterExpression,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_key_relative_index_range(
                    core_return_type,
                    key._as,
                    index._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items nearest to key and greater by index with a count limit
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_key_relative_index_range_count(
            return_type: MapReturnType,
            key: FilterExpression,
            index: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_key_relative_index_range_count(
                    core_return_type,
                    key._as,
                    index._as,
                    count._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items nearest to value and greater by relative rank
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_value_relative_rank_range(
            return_type: MapReturnType,
            value: FilterExpression,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_value_relative_rank_range(
                    core_return_type,
                    value._as,
                    rank._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items nearest to value and greater by relative rank with a count limit
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_value_relative_rank_range_count(
            return_type: MapReturnType,
            value: FilterExpression,
            rank: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_value_relative_rank_range_count(
                    core_return_type,
                    value._as,
                    rank._as,
                    count._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items starting at specified index to the end of map
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_index_range(
            return_type: MapReturnType,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_index_range(
                    core_return_type,
                    index._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects "count" map items starting at specified index
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_index_range_count(
            return_type: MapReturnType,
            index: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_index_range_count(
                    core_return_type,
                    index._as,
                    count._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects map items starting at specified rank to the last ranked item
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_rank_range(
            return_type: MapReturnType,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_rank_range(
                    core_return_type,
                    rank._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that selects "count" map items starting at specified rank
        /// and returns selected data specified by returnType.
        /// Supports nested CDT operations via optional CTX contexts.
        pub fn map_get_by_rank_range_count(
            return_type: MapReturnType,
            rank: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            let ctx_vec = crate::cdt::ctx_to_vec(&ctx);
            let core_return_type = return_type.into();
            FilterExpression {
                _as: maps::get_by_rank_range_count(
                    core_return_type,
                    rank._as,
                    count._as,
                    bin._as,
                    &ctx_vec,
                ),
            }
        }

        //--------------------------------------------------
        // Map CDT Write Expressions
        //--------------------------------------------------

        #[staticmethod]
        /// Create expression that writes key/value item to map bin.
        pub fn map_put(
            policy: MapPolicy,
            key: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::put(&policy._as, key._as, value._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that writes each map item to map bin.
        pub fn map_put_items(
            policy: MapPolicy,
            map: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::put_items(&policy._as, map._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that increments values by incr for all items identified by key.
        pub fn map_increment(
            policy: MapPolicy,
            key: FilterExpression,
            incr: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::increment(&policy._as, key._as, incr._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        //--------------------------------------------------
        // Map CDT Remove Expressions
        //--------------------------------------------------

        #[staticmethod]
        /// Create expression that removes all items in map.
        pub fn map_clear(bin: FilterExpression, ctx: Vec<CTX>) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::clear(bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes map item identified by key.
        pub fn map_remove_by_key(
            return_type: MapReturnType,
            key: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_key(return_type.into(), key._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items identified by keys.
        pub fn map_remove_by_key_list(
            return_type: MapReturnType,
            keys: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_key_list(return_type.into(), keys._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items identified by key range
        /// (key_begin inclusive, key_end exclusive).
        pub fn map_remove_by_key_range(
            return_type: MapReturnType,
            key_begin: Option<FilterExpression>,
            key_end: Option<FilterExpression>,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_key_range(
                    return_type.into(),
                    key_begin.map(|e| e._as),
                    key_end.map(|e| e._as),
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items nearest to key and greater by index.
        pub fn map_remove_by_key_relative_index_range(
            return_type: MapReturnType,
            key: FilterExpression,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_key_relative_index_range(
                    return_type.into(),
                    key._as,
                    index._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items nearest to key and greater by index
        /// with a count limit.
        pub fn map_remove_by_key_relative_index_range_count(
            return_type: MapReturnType,
            key: FilterExpression,
            index: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_key_relative_index_range_count(
                    return_type.into(),
                    key._as,
                    index._as,
                    count._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items identified by value.
        pub fn map_remove_by_value(
            return_type: MapReturnType,
            value: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_value(return_type.into(), value._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items identified by values.
        pub fn map_remove_by_value_list(
            return_type: MapReturnType,
            values: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_value_list(return_type.into(), values._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items identified by value range
        /// (value_begin inclusive, value_end exclusive).
        pub fn map_remove_by_value_range(
            return_type: MapReturnType,
            value_begin: Option<FilterExpression>,
            value_end: Option<FilterExpression>,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_value_range(
                    return_type.into(),
                    value_begin.map(|e| e._as),
                    value_end.map(|e| e._as),
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items nearest to value and greater by relative rank.
        pub fn map_remove_by_value_relative_rank_range(
            return_type: MapReturnType,
            value: FilterExpression,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_value_relative_rank_range(
                    return_type.into(),
                    value._as,
                    rank._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items nearest to value and greater by relative rank
        /// with a count limit.
        pub fn map_remove_by_value_relative_rank_range_count(
            return_type: MapReturnType,
            value: FilterExpression,
            rank: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_value_relative_rank_range_count(
                    return_type.into(),
                    value._as,
                    rank._as,
                    count._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes map item identified by index.
        pub fn map_remove_by_index(
            return_type: MapReturnType,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_index(return_type.into(), index._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items starting at specified index to the end of map.
        pub fn map_remove_by_index_range(
            return_type: MapReturnType,
            index: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_index_range(return_type.into(), index._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes "count" map items starting at specified index.
        pub fn map_remove_by_index_range_count(
            return_type: MapReturnType,
            index: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_index_range_count(
                    return_type.into(),
                    index._as,
                    count._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        #[staticmethod]
        /// Create expression that removes map item identified by rank.
        pub fn map_remove_by_rank(
            return_type: MapReturnType,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_rank(return_type.into(), rank._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes map items starting at specified rank to the last ranked item.
        pub fn map_remove_by_rank_range(
            return_type: MapReturnType,
            rank: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_rank_range(return_type.into(), rank._as, bin._as, &crate::cdt::ctx_to_vec(&ctx)),
            }
        }

        #[staticmethod]
        /// Create expression that removes "count" map items starting at specified rank.
        pub fn map_remove_by_rank_range_count(
            return_type: MapReturnType,
            rank: FilterExpression,
            count: FilterExpression,
            bin: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            use aerospike_core::expressions::maps;
            FilterExpression {
                _as: maps::remove_by_rank_range_count(
                    return_type.into(),
                    rank._as,
                    count._as,
                    bin._as,
                    &crate::cdt::ctx_to_vec(&ctx),
                ),
            }
        }

        //--------------------------------------------------
        // Bitwise Expressions
        //--------------------------------------------------

        #[staticmethod]
        /// Create expression that resizes byte[] to byte_size according to resize_flags
        /// and returns byte[].
        pub fn bit_resize(
            policy: BitPolicy,
            byte_size: FilterExpression,
            resize_flags: BitwiseResizeFlags,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::resize(&policy._as, byte_size._as, resize_flags.into(), bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that inserts value bytes into byte[] bin at byte_offset
        /// and returns byte[].
        pub fn bit_insert(
            policy: BitPolicy,
            byte_offset: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::insert(&policy._as, byte_offset._as, value._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that removes bytes from byte[] bin at byte_offset for byte_size
        /// and returns byte[].
        pub fn bit_remove(
            policy: BitPolicy,
            byte_offset: FilterExpression,
            byte_size: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::remove(&policy._as, byte_offset._as, byte_size._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that sets value on byte[] bin at bit_offset for bit_size
        /// and returns byte[].
        pub fn bit_set(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::set(&policy._as, bit_offset._as, bit_size._as, value._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that performs bitwise "or" on value and byte[] bin at bit_offset
        /// for bit_size and returns byte[].
        pub fn bit_or(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::or(&policy._as, bit_offset._as, bit_size._as, value._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that performs bitwise "xor" on value and byte[] bin at bit_offset
        /// for bit_size and returns byte[].
        pub fn bit_xor(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::xor(&policy._as, bit_offset._as, bit_size._as, value._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that performs bitwise "and" on value and byte[] bin at bit_offset
        /// for bit_size and returns byte[].
        pub fn bit_and(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::and(&policy._as, bit_offset._as, bit_size._as, value._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that negates byte[] bin starting at bit_offset for bit_size
        /// and returns byte[].
        pub fn bit_not(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::not(&policy._as, bit_offset._as, bit_size._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that shifts left byte[] bin starting at bit_offset for bit_size
        /// and returns byte[].
        pub fn bit_lshift(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            shift: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::lshift(&policy._as, bit_offset._as, bit_size._as, shift._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that shifts right byte[] bin starting at bit_offset for bit_size
        /// and returns byte[].
        pub fn bit_rshift(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            shift: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::rshift(&policy._as, bit_offset._as, bit_size._as, shift._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that adds value to byte[] bin starting at bit_offset for bit_size
        /// and returns byte[]. BitSize must be <= 64.
        pub fn bit_add(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            value: FilterExpression,
            signed: bool,
            action: BitwiseOverflowActions,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::add(
                    &policy._as,
                    bit_offset._as,
                    bit_size._as,
                    value._as,
                    signed,
                    action.into(),
                    bin._as,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that subtracts value from byte[] bin starting at bit_offset for bit_size
        /// and returns byte[]. BitSize must be <= 64.
        pub fn bit_subtract(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            value: FilterExpression,
            signed: bool,
            action: BitwiseOverflowActions,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::subtract(
                    &policy._as,
                    bit_offset._as,
                    bit_size._as,
                    value._as,
                    signed,
                    action.into(),
                    bin._as,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that sets value to byte[] bin starting at bit_offset for bit_size
        /// and returns byte[]. BitSize must be <= 64.
        pub fn bit_set_int(
            policy: BitPolicy,
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::set_int(&policy._as, bit_offset._as, bit_size._as, value._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns bits from byte[] bin starting at bit_offset for bit_size.
        pub fn bit_get(
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::get(bit_offset._as, bit_size._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns integer count of set bits from byte[] bin starting at
        /// bit_offset for bit_size.
        pub fn bit_count(
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::count(bit_offset._as, bit_size._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns integer bit offset of the first specified value bit
        /// in byte[] bin starting at bit_offset for bit_size.
        pub fn bit_lscan(
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::lscan(bit_offset._as, bit_size._as, value._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns integer bit offset of the last specified value bit
        /// in byte[] bin starting at bit_offset for bit_size.
        pub fn bit_rscan(
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            value: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::rscan(bit_offset._as, bit_size._as, value._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns integer from byte[] bin starting at bit_offset for bit_size.
        /// Signed indicates if bits should be treated as a signed number.
        pub fn bit_get_int(
            bit_offset: FilterExpression,
            bit_size: FilterExpression,
            signed: bool,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::get_int(bit_offset._as, bit_size._as, signed, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns the base64 text of the whole byte[] bin.
        /// Requires Aerospike Server version 8.2.0 or later.
        pub fn bit_b64_encode(bin: FilterExpression) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::b64_encode(bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns the base64 text of a byte range of byte[] bin,
        /// starting at byte_offset for byte_size. When invert_size is true, byte_size
        /// counts back from the end of the blob, so an inverted size of 0 encodes
        /// through to the end. Requires Aerospike Server version 8.2.0 or later.
        pub fn bit_b64_encode_range(
            byte_offset: FilterExpression,
            byte_size: FilterExpression,
            invert_size: bool,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::bitwise;
            FilterExpression {
                _as: bitwise::b64_encode_range(byte_offset._as, byte_size._as, invert_size, bin._as),
            }
        }

        //--------------------------------------------------
        // HLL Expressions
        //--------------------------------------------------

        #[staticmethod]
        /// Create expression that creates a new HLL or resets an existing HLL.
        pub fn hll_init(
            policy: HLLPolicy,
            index_bit_count: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::init(policy._as, index_bit_count._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that creates a new HLL or resets an existing HLL with minhash bits.
        pub fn hll_init_with_min_hash(
            policy: HLLPolicy,
            index_bit_count: FilterExpression,
            min_hash_count: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::init_with_min_hash(policy._as, index_bit_count._as, min_hash_count._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that adds list values to a HLL set and returns HLL set.
        pub fn hll_add(
            policy: HLLPolicy,
            list: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::add(policy._as, list._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that adds values to a HLL set and returns HLL set.
        /// If HLL bin does not exist, use index_bit_count to create HLL bin.
        pub fn hll_add_with_index(
            policy: HLLPolicy,
            list: FilterExpression,
            index_bit_count: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::add_with_index(policy._as, list._as, index_bit_count._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that adds values to a HLL set and returns HLL set.
        /// If HLL bin does not exist, use index_bit_count and min_hash_count to create HLL set.
        pub fn hll_add_with_index_and_min_hash(
            policy: HLLPolicy,
            list: FilterExpression,
            index_bit_count: FilterExpression,
            min_hash_count: FilterExpression,
            bin: FilterExpression,
        ) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::add_with_index_and_min_hash(
                    policy._as,
                    list._as,
                    index_bit_count._as,
                    min_hash_count._as,
                    bin._as,
                ),
            }
        }

        #[staticmethod]
        /// Create expression that returns estimated number of elements in the HLL bin.
        pub fn hll_get_count(bin: FilterExpression) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::get_count(bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns a HLL object that is the union of all specified
        /// HLL objects in the list with the HLL bin.
        pub fn hll_get_union(list: FilterExpression, bin: FilterExpression) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::get_union(list._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns estimated number of elements that would be contained
        /// by the union of these HLL objects.
        pub fn hll_get_union_count(list: FilterExpression, bin: FilterExpression) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::get_union_count(list._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns estimated number of elements that would be contained
        /// by the intersection of these HLL objects.
        pub fn hll_get_intersect_count(list: FilterExpression, bin: FilterExpression) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::get_intersect_count(list._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns estimated similarity of these HLL objects
        /// as a 64 bit float.
        pub fn hll_get_similarity(list: FilterExpression, bin: FilterExpression) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::get_similarity(list._as, bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns index_bit_count and min_hash_bit_count used to create
        /// HLL bin in a list of longs.
        pub fn hll_describe(bin: FilterExpression) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::describe(bin._as),
            }
        }

        #[staticmethod]
        /// Create expression that returns one if HLL bin may contain all items in the list.
        pub fn hll_may_contain(list: FilterExpression, bin: FilterExpression) -> Self {
            use aerospike_core::expressions::hll;
            FilterExpression {
                _as: hll::may_contain(list._as, bin._as),
            }
        }

        ////////////////////////////////////////////////////////////////////////////////////////
        //  Loop variable expressions (path expressions, server >= 8.1.1)
        ////////////////////////////////////////////////////////////////////////////////////////

        #[staticmethod]
        /// Create a boolean loop variable expression for use in path expressions.
        ///
        /// Args:
        ///     part: Which element of the loop variable to access (``LoopVarPart``).
        ///
        /// Returns:
        ///     A ``FilterExpression`` representing the boolean loop variable.
        pub fn bool_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_bool_loop_var(part.into()),
            }
        }

        #[staticmethod]
        /// Create an integer loop variable expression for use in path expressions.
        ///
        /// Args:
        ///     part: Which element of the loop variable to access (``LoopVarPart``).
        ///
        /// Returns:
        ///     A ``FilterExpression`` representing the integer loop variable.
        pub fn int_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_int_loop_var(part.into()),
            }
        }

        #[staticmethod]
        /// Create a float loop variable expression for use in path expressions.
        ///
        /// Args:
        ///     part: Which element of the loop variable to access (``LoopVarPart``).
        ///
        /// Returns:
        ///     A ``FilterExpression`` representing the float loop variable.
        pub fn float_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_float_loop_var(part.into()),
            }
        }

        #[staticmethod]
        /// Create a string loop variable expression for use in path expressions.
        ///
        /// Args:
        ///     part: Which element of the loop variable to access (``LoopVarPart``).
        ///
        /// Returns:
        ///     A ``FilterExpression`` representing the string loop variable.
        pub fn string_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_string_loop_var(part.into()),
            }
        }

        #[staticmethod]
        /// Create a list loop variable expression for use in path expressions.
        ///
        /// Args:
        ///     part: Which element of the loop variable to access (``LoopVarPart``).
        ///
        /// Returns:
        ///     A ``FilterExpression`` representing the list loop variable.
        pub fn list_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_list_loop_var(part.into()),
            }
        }

        #[staticmethod]
        /// Create a map loop variable expression for use in path expressions.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        pub fn map_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_map_loop_var(part.into()),
            }
        }

        /// Retrieve the blob part of the current loop variable.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn blob_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_blob_loop_var(part.into()),
            }
        }

        /// Retrieve the HLL part of the current loop variable.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn hll_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_hll_loop_var(part.into()),
            }
        }

        /// Retrieve the nil part of the current loop variable.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn nil_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_nil_loop_var(part.into()),
            }
        }

        /// Retrieve the GeoJSON part of the current loop variable.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn geo_json_loop_var(part: &crate::enums::LoopVarPart) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_geo_json_loop_var(part.into()),
            }
        }

        /// Signal that the current loop element should be removed from the result.
        ///
        /// Used as the modify expression in :meth:`CdtOperation.modify_by_path` to
        /// delete matched elements rather than replace them.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn remove_result() -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::exp_remove_result(),
            }
        }

        // ===== Native ExpOps from server 8.1.2 (CLIENT-4437) =====
        //
        // These are wire-level opcodes — single ExpOp invocations rather than
        // compositions of existing list/map ops. Cheaper to pack and evaluate
        // than the prior compositional shims.

        /// ``value`` exists in ``list``. Native ExpOp on server 8.1.2+ —
        /// returns a boolean expression equivalent to ``value IN list``.
        ///
        /// Requires Aerospike Server version >= 8.1.2.
        #[staticmethod]
        pub fn in_list(value: FilterExpression, list: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::in_list(value._as, list._as),
            }
        }

        /// All keys of a map expression as a list expression. Native ExpOp on
        /// server 8.1.2+ (cheaper than ``map_get_by_index_range(KEY, 0, ...)``).
        ///
        /// Requires Aerospike Server version >= 8.1.2.
        #[staticmethod]
        pub fn map_keys(map: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::map_keys(map._as),
            }
        }

        /// All values of a map expression as a list expression. Native ExpOp
        /// on server 8.1.2+ (cheaper than ``map_get_by_index_range(VALUE, 0, ...)``).
        ///
        /// Requires Aerospike Server version >= 8.1.2.
        #[staticmethod]
        pub fn map_values(map: FilterExpression) -> Self {
            FilterExpression {
                _as: aerospike_core::expressions::map_values(map._as),
            }
        }

        // ===== Path-based expression operators (CLIENT-4437) =====
        //
        // Read or write nested CDT data at a path context, evaluating to an
        // expression result. Mirrors :meth:`CdtOperation.select_by_path` /
        // :meth:`CdtOperation.modify_by_path` but produces an
        // ``Expression`` (not an ``Operation``) usable inside other
        // expressions.

        /// Select from a CDT bin expression using a path context. The
        /// ``flag`` is a bitwise combination of ``SelectFlags`` constants.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn exp_select_by_path(
            return_type: ExpType,
            flag: i64,
            bin_exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: aerospike_core::expressions::exp_select_by_path(
                    (&return_type).into(),
                    aerospike_core::operations::path::SelectFlag::from_bits(flag),
                    bin_exp._as,
                    &core_ctx,
                ),
            }
        }

        /// Modify a CDT bin expression using a path context. The ``flag`` is
        /// a bitwise combination of ``ModifyFlags`` constants.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn exp_modify_by_path(
            return_type: ExpType,
            flag: i64,
            bin_exp: FilterExpression,
            modify_exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: aerospike_core::expressions::exp_modify_by_path(
                    (&return_type).into(),
                    aerospike_core::operations::path::ModifyFlag::from_bits(flag),
                    bin_exp._as,
                    modify_exp._as,
                    &core_ctx,
                ),
            }
        }

        /// Convenience wrapper: select the *values* at every path-resolved
        /// location (``SelectFlags.VALUE``).
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn exp_select_values(
            return_type: ExpType,
            bin_exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: aerospike_core::expressions::exp_select_values(
                    (&return_type).into(),
                    bin_exp._as,
                    &core_ctx,
                ),
            }
        }

        /// Convenience wrapper: select map *keys* at every path-resolved
        /// location (``SelectFlags.MAP_KEY``).
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn exp_select_map_keys(
            return_type: ExpType,
            bin_exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: aerospike_core::expressions::exp_select_map_keys(
                    (&return_type).into(),
                    bin_exp._as,
                    &core_ctx,
                ),
            }
        }

        /// Convenience wrapper: select map *key/value pairs*
        /// (``SelectFlags.MAP_KEY_VALUE``).
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn exp_select_map_entries(
            return_type: ExpType,
            bin_exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: aerospike_core::expressions::exp_select_map_entries(
                    (&return_type).into(),
                    bin_exp._as,
                    &core_ctx,
                ),
            }
        }

        /// Convenience wrapper: select the *original tree shape* preserving
        /// only matching nodes (``SelectFlags.MATCHING_TREE``).
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn exp_select_matching_tree(
            return_type: ExpType,
            bin_exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: aerospike_core::expressions::exp_select_matching_tree(
                    (&return_type).into(),
                    bin_exp._as,
                    &core_ctx,
                ),
            }
        }

        /// Convenience wrapper: modify with default flags, failing on type
        /// mismatches (``ModifyFlags.DEFAULT``).
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn exp_modify(
            return_type: ExpType,
            bin_exp: FilterExpression,
            modify_exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: aerospike_core::expressions::exp_modify(
                    (&return_type).into(),
                    bin_exp._as,
                    modify_exp._as,
                    &core_ctx,
                ),
            }
        }

        /// Convenience wrapper: modify with ``ModifyFlags.NO_FAIL`` so
        /// type-mismatched leaves are silently skipped instead of aborting
        /// the whole expression.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn exp_modify_no_fail(
            return_type: ExpType,
            bin_exp: FilterExpression,
            modify_exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: aerospike_core::expressions::exp_modify_no_fail(
                    (&return_type).into(),
                    bin_exp._as,
                    modify_exp._as,
                    &core_ctx,
                ),
            }
        }

        /// Convenience wrapper: remove the leaves resolved by a path.
        /// Equivalent to ``exp_modify_by_path(t, ModifyFlags.DEFAULT, bin, FilterExpression.remove_result(), ctx)``.
        ///
        /// Requires Aerospike Server version >= 8.1.1.
        #[staticmethod]
        pub fn exp_remove(
            return_type: ExpType,
            bin_exp: FilterExpression,
            ctx: Vec<CTX>,
        ) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            FilterExpression {
                _as: aerospike_core::expressions::exp_remove(
                    (&return_type).into(),
                    bin_exp._as,
                    &core_ctx,
                ),
            }
        }

        ////////////////////////////////////////////////////////////////////////////////////////////
        //
        //  String expressions (server 8.2.0+)
        //
        //  Wraps aerospike-core/src/expressions/string.rs. Conventions:
        //    - `src` is the TRAILING argument (matches the existing `bit_*` /
        //      `hll_*` expression conventions in this file, and the spec §3.7
        //      argument-shape rule).
        //    - No `ctx` parameter — string expressions don't take CdtContext directly.
        //      To target a string nested inside a list/map, project via
        //      `list_get_by_index(VALUE, STRING, …)` or
        //      `map_get_by_key(VALUE, STRING, …)` first.
        //    - Modify expressions return the modified string as an Exp value;
        //      they do NOT mutate the bin.
        //    - Per-op `flags` argument is u8 (StringWriteFlags / StringRegexFlags),
        //      converted to rust-core's i64-backed tuple structs inline at zero cost.
        //
        ////////////////////////////////////////////////////////////////////////////////////////////

        // ---- Read expressions ----

        #[staticmethod]
        /// Codepoint count of `src` as an INT (NOT byte count — use `string_byte_length`).
        pub fn string_strlen(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::strlen(src._as) }
        }

        #[staticmethod]
        /// Substring of `src` from codepoint `start` to the end.
        pub fn string_substr(start: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::substr(src._as, start._as) }
        }

        #[staticmethod]
        /// Substring of `src` over the half-open codepoint range ``[start, end)``.
        /// The second arg is named ``end`` (exclusive index) to reflect the
        /// server's actual decoder behavior — rust-core's parameter name
        /// "length" in its docstring is misleading.
        pub fn string_substr_range(start: FilterExpression, end: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::substr_range(src._as, start._as, end._as) }
        }

        #[staticmethod]
        /// Codepoint at `index` (one-codepoint string).
        pub fn string_char_at(index: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::char_at(src._as, index._as) }
        }

        #[staticmethod]
        /// First-match codepoint index of `needle` in `src` (-1 if absent).
        pub fn string_find(needle: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::find(src._as, needle._as) }
        }

        #[staticmethod]
        /// N-th-match codepoint index of `needle` in `src` (1 = first, -1 = last; -1 if absent).
        pub fn string_find_nth(needle: FilterExpression, occurrence: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::find_nth(src._as, needle._as, occurrence._as) }
        }

        #[staticmethod]
        /// Returns BOOL — `src` contains `needle` as a substring.
        pub fn string_contains(needle: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::contains(src._as, needle._as) }
        }

        #[staticmethod]
        /// Returns BOOL — `src` starts with `prefix`.
        pub fn string_starts_with(prefix: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::starts_with(src._as, prefix._as) }
        }

        #[staticmethod]
        /// Returns BOOL — `src` ends with `suffix`.
        pub fn string_ends_with(suffix: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::ends_with(src._as, suffix._as) }
        }

        #[staticmethod]
        /// Parse `src` as INT. Returns OP_NOT_APPLICABLE (subcode
        /// `OPNOT_STRING_CONVERSION_FAILED`) on unparseable input.
        pub fn string_to_integer(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::to_integer(src._as) }
        }

        #[staticmethod]
        /// Parse `src` as FLOAT (f64). Returns OP_NOT_APPLICABLE (subcode
        /// `OPNOT_STRING_CONVERSION_FAILED`) on unparseable input.
        pub fn string_to_double(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::to_double(src._as) }
        }

        #[staticmethod]
        /// UTF-8 byte length of `src` (differs from `string_strlen` for non-ASCII).
        pub fn string_byte_length(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::byte_length(src._as) }
        }

        #[staticmethod]
        /// Returns BOOL — `src` parses as a number (integer or float).
        pub fn string_is_numeric(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::is_numeric(src._as) }
        }

        #[staticmethod]
        /// Returns BOOL — `src` parses as the requested numeric type.
        pub fn string_is_numeric_typed(numeric_type: StringNumericType, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::StringNumericType as CoreNT;
            let core_nt = match numeric_type {
                StringNumericType::Any => CoreNT::Any,
                StringNumericType::Int => CoreNT::Int,
                StringNumericType::Float => CoreNT::Float,
            };
            FilterExpression { _as: str_exp::is_numeric_typed(src._as, core_nt) }
        }

        #[staticmethod]
        /// Returns BOOL — every cased codepoint in `src` is uppercase.
        pub fn string_is_upper(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::is_upper(src._as) }
        }

        #[staticmethod]
        /// Returns BOOL — every cased codepoint in `src` is lowercase.
        pub fn string_is_lower(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::is_lower(src._as) }
        }

        #[staticmethod]
        /// Returns BLOB — UTF-8 bytes of `src`.
        pub fn string_to_blob(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::to_blob(src._as) }
        }

        #[staticmethod]
        /// Returns LIST — `src` split by codepoint (one element per codepoint).
        pub fn string_split(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::split(src._as) }
        }

        #[staticmethod]
        /// Returns LIST — `src` split by `separator`.
        pub fn string_split_by_separator(separator: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::split_by_separator(src._as, separator._as) }
        }

        #[staticmethod]
        /// Returns BLOB — `src` treated as base64-encoded text, decoded to bytes.
        /// Invalid base64 returns OP_NOT_APPLICABLE (subcode `OPNOT_STRING_B64_INVALID`).
        pub fn string_b64_decode(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::b64_decode(src._as) }
        }

        #[staticmethod]
        /// Returns BOOL — `src` matches ICU regex `pattern`. Use
        /// `string_regex_compare_with_flags` to pass case-insensitive etc.
        /// flags.
        ///
        /// Server-side limitation (spec §4.2): the expression engine does
        /// NOT honor a literal source via ``Exp.val(...)`` —
        /// ``string_regex_compare(Exp.val("pat"), Exp.val("text"))`` returns
        /// OP_NOT_APPLICABLE (26). Only bin-sourced inputs are verified.
        pub fn string_regex_compare(pattern: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::regex_compare(src._as, pattern._as) }
        }

        #[staticmethod]
        /// Returns BOOL — `src` matches ICU regex `pattern` with the given
        /// `regex_flags` (OR-combined `StringRegexFlags` bitmask).
        pub fn string_regex_compare_with_flags(pattern: FilterExpression, regex_flags: u8, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::StringRegexFlags as CoreSRF;
            FilterExpression {
                _as: str_exp::regex_compare_with_flags(src._as, pattern._as, CoreSRF::from_bits(regex_flags as i64)),
            }
        }

        // ---- Modify expressions (return modified string VALUE; do not persist) ----
        //
        // Each takes a u8 `flags` (StringWriteFlags bitmask) wrapped into a
        // rust-core StringPolicy inline. Cost: one stack StringPolicy +
        // one stack StringWriteFlags per call — zero heap.

        #[staticmethod]
        /// Returns STRING — `src` with `value` spliced in at codepoint `index`.
        pub fn string_insert(flags: u8, index: FilterExpression, value: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::insert(&policy, src._as, index._as, value._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with codepoints starting at `index` overwritten by `value`.
        pub fn string_overwrite(flags: u8, index: FilterExpression, value: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::overwrite(&policy, src._as, index._as, value._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` concatenated with the LIST-yielding `values` expression.
        /// Per spec §3.7 the expression-path `concat` always takes a list source;
        /// single-string callers must wrap via ``FilterExpression.list_val([s])``.
        pub fn string_concat(flags: u8, values: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::concat(&policy, src._as, values._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with `value` joined onto its end.
        pub fn string_append(flags: u8, value: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::append(&policy, src._as, value._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with `value` joined onto its front.
        pub fn string_prepend(flags: u8, value: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::prepend(&policy, src._as, value._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with the half-open codepoint range ``[start, end)`` removed.
        /// Use ``string_snip_from`` to snip from `start` through the end.
        pub fn string_snip(flags: u8, start: FilterExpression, end: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::snip(&policy, src._as, start._as, end._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with codepoints from `start` through the end
        /// removed (truncate-to-end). Takes no write flags: the server reads
        /// the snip arguments by position — `start`, `end`, then flags — so
        /// this form packs ``[53, start]`` only; use ``string_snip`` with an
        /// explicit `end` when the flags have to be honored.
        pub fn string_snip_from(start: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::StringPolicy;
            FilterExpression { _as: str_exp::snip_from(&StringPolicy::default(), src._as, start._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with the first occurrence of `needle` replaced by `replacement`.
        pub fn string_replace(flags: u8, needle: FilterExpression, replacement: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::replace(&policy, src._as, needle._as, replacement._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with every occurrence of `needle` replaced by `replacement`.
        pub fn string_replace_all(flags: u8, needle: FilterExpression, replacement: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::replace_all(&policy, src._as, needle._as, replacement._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` uppercased.
        pub fn string_upper(flags: u8, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            FilterExpression { _as: str_exp::upper(&StringPolicy::new(CoreSWF::from_bits(flags as i64)), src._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` lowercased.
        pub fn string_lower(flags: u8, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            FilterExpression { _as: str_exp::lower(&StringPolicy::new(CoreSWF::from_bits(flags as i64)), src._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with locale-independent case fold applied.
        pub fn string_case_fold(flags: u8, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            FilterExpression { _as: str_exp::case_fold(&StringPolicy::new(CoreSWF::from_bits(flags as i64)), src._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` normalized to Unicode NFC form.
        pub fn string_normalize_nfc(flags: u8, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            FilterExpression { _as: str_exp::normalize_nfc(&StringPolicy::new(CoreSWF::from_bits(flags as i64)), src._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with whitespace stripped from the start.
        pub fn string_trim_start(flags: u8, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            FilterExpression { _as: str_exp::trim_start(&StringPolicy::new(CoreSWF::from_bits(flags as i64)), src._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with whitespace stripped from the end.
        pub fn string_trim_end(flags: u8, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            FilterExpression { _as: str_exp::trim_end(&StringPolicy::new(CoreSWF::from_bits(flags as i64)), src._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with whitespace stripped from both ends.
        pub fn string_trim(flags: u8, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            FilterExpression { _as: str_exp::trim(&StringPolicy::new(CoreSWF::from_bits(flags as i64)), src._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` left-padded with `pad_string` to `target_length` codepoints.
        pub fn string_pad_start(flags: u8, target_length: FilterExpression, pad_string: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::pad_start(&policy, src._as, target_length._as, pad_string._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` right-padded with `pad_string` to `target_length` codepoints.
        pub fn string_pad_end(flags: u8, target_length: FilterExpression, pad_string: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::pad_end(&policy, src._as, target_length._as, pad_string._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` contents repeated `count` times.
        pub fn string_repeat(flags: u8, count: FilterExpression, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringWriteFlags as CoreSWF};
            let policy = StringPolicy::new(CoreSWF::from_bits(flags as i64));
            FilterExpression { _as: str_exp::repeat(&policy, src._as, count._as) }
        }

        #[staticmethod]
        /// Returns STRING — `src` with the first match of `pattern` replaced by
        /// `replacement`. Set the `GLOBAL` bit in `regex_flags` to replace every match.
        ///
        /// Note: rust-core's signature includes `_policy` for API symmetry; the
        /// wire payload has no write-flags slot (the server rejects messages
        /// that pack one). PNC passes the default policy and surfaces only
        /// `regex_flags` here.
        pub fn string_regex_replace(pattern: FilterExpression, replacement: FilterExpression, regex_flags: u8, src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            use aerospike_core::operations::string::{StringPolicy, StringRegexFlags as CoreSRF};
            let policy = StringPolicy::default();
            FilterExpression {
                _as: str_exp::regex_replace(&policy, src._as, pattern._as, replacement._as, CoreSRF::from_bits(regex_flags as i64)),
            }
        }

        // ---- Type conversion ----

        #[staticmethod]
        #[pyo3(name = "to_string")]
        /// Returns STRING — `src` (bool / integer / float / string / blob) coerced to its string
        /// representation. Unlike the other string expressions, which dispatch through the
        /// CALL_STRING module (id 3), this is encoded as the dedicated unary TO_STRING
        /// expression opcode.
        pub fn to_string_expr(src: FilterExpression) -> Self {
            use aerospike_core::expressions::string as str_exp;
            FilterExpression { _as: str_exp::to_string(src._as) }
        }
    }
