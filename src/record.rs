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

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::ffi::{c_int, c_void, CStr};
use std::fmt;
use std::hash::{Hash, Hasher};

use pyo3::basic::CompareOp;
use pyo3::buffer::{ElementType, PyUntypedBuffer};
use pyo3::exceptions::{PyBufferError, PyIndexError, PyValueError};
use pyo3::exceptions::PyTypeError;
use pyo3::ffi;
use pyo3::types::{
    PyBool, PyByteArray, PyBytes, PyDict, PyIterator, PyList, PyMemoryView, PySlice, PySliceMethods, PySuper,
    PyTuple,
};
use pyo3::PyTypeInfo;
use pyo3::{prelude::*, Borrowed, IntoPyObjectExt};

use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};
use pyo3_stub_gen::{PyStubType, TypeInfo};

use aerospike_core::VectorData;

use crate::errors::RustClientError;


    /**********************************************************************************
     *
     * Record
     *
     **********************************************************************************/

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    pub struct Record {
        pub(crate) _as: aerospike_core::Record,
        /// Lazily-cached Python dict for the ``bins`` property.
        /// Avoids re-cloning and re-converting on every access.
        pub(crate) cached_bins: Option<Py<PyAny>>,
        /// Lazily-cached Python list for the ``results`` property.
        pub(crate) cached_results: Option<Py<PyAny>>,
    }

    impl Clone for Record {
        fn clone(&self) -> Self {
            Record {
                _as: self._as.clone(),
                // Don't carry caches across clones; the new owner
                // will lazily rebuild them if needed.
                cached_bins: None,
                cached_results: None,
            }
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Record {
        pub fn bin(&self, name: &str) -> Option<Py<PyAny>> {
            let b = self._as.bins.get(name);
            b.map(|v| {
                let v: PythonValue = v.to_owned().into();
                Python::attach(|py| v.into_pyobject(py).unwrap().unbind())
            })
        }

        #[getter]
        pub fn get_bins(&mut self, py: Python<'_>) -> Py<PyAny> {
            if let Some(ref cached) = self.cached_bins {
                return cached.clone_ref(py);
            }
            let dict = PyDict::new(py);
            for (k, v) in &self._as.bins {
                let pv: PythonValue = v.clone().into();
                let py_val = pv.into_pyobject(py).unwrap();
                dict.set_item(k, py_val).unwrap();
            }
            let py_obj: Py<PyAny> = dict.into_any().unbind();
            self.cached_bins = Some(py_obj.clone_ref(py));
            py_obj
        }

        /// Positional results, one slot per op in request order.
        ///
        /// Use ``record.results`` when the request issued multiple ops and you
        /// need to address each result by its op index — e.g. a chain that
        /// modifies a bin and reads it back in the same execute. Slots for
        /// ops that produced no value carry Python ``None``.
        ///
        /// For by-name access, prefer ``record.bins`` (cheaper for the common
        /// case of one op per bin).
        #[getter]
        pub fn get_results(&mut self, py: Python<'_>) -> Option<Py<PyAny>> {
            if let Some(ref cached) = self.cached_results {
                return Some(cached.clone_ref(py));
            }
            let results = self._as.results.as_ref()?;
            let list = PyList::empty(py);
            for v in results {
                let pv: PythonValue = v.clone().into();
                let py_val = pv.into_pyobject(py).unwrap();
                list.append(py_val).unwrap();
            }
            let py_obj: Py<PyAny> = list.into_any().unbind();
            self.cached_results = Some(py_obj.clone_ref(py));
            Some(py_obj)
        }

        /// Return the positional result for the *i*-th op in the request,
        /// or ``None`` if *i* is out of range.
        ///
        /// Equivalent to ``record.results[i]`` but without bounds-error noise:
        /// out-of-range returns Python ``None`` (matching the in-range
        /// nil-result encoding). Use this when the request size is dynamic
        /// and the caller wants a uniform optional shape.
        pub fn operation_result(&self, i: usize) -> Option<Py<PyAny>> {
            self._as.operation_result(i).map(|v| {
                let pv: PythonValue = v.to_owned().into();
                Python::attach(|py| pv.into_pyobject(py).unwrap().unbind())
            })
        }

        #[getter]
        pub fn get_generation(&self) -> Option<u32> {
            Some(self._as.generation)
        }

        #[getter]
        pub fn get_ttl(&self) -> Option<u32> {
            self._as.time_to_live().map(|v| v.as_secs() as u32)
        }

        #[getter]
        pub fn get_key(&self) -> Option<Key> {
            self._as.key.as_ref().map(|k| Key { _as: k.clone() })
        }

        fn __str__(&self) -> PyResult<String> {
            Ok(format!("{}", self))
        }

        fn __repr__(&self) -> PyResult<String> {
            let s = self.__str__()?;
            Ok(format!("Record({})", s))
        }
    }

    impl fmt::Display for Record {
        fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(f, "generation: {}", self._as.generation)?;
            write!(f, ", ttl: ")?;
            let _ = match self._as.time_to_live() {
                None => "None".fmt(f),
                Some(duration) => duration.as_secs().fmt(f),
            };
            write!(f, ", key: {:?}", self._as.key)?;
            write!(f, ", bins: {{")?;
            for (i, (k, v)) in self._as.bins.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "'{}': {}", k, v)?;
            }
            write!(f, "}}")?;
            Ok(())
        }
    }

    /**********************************************************************************
     *
     * Key
     *
     **********************************************************************************/

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone)]
    pub struct Key {
        pub(crate) _as: aerospike_core::Key,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Key {
        #[new]
        fn new(namespace: &str, set: &str, key: PythonValue) -> Self {
            // Pass key value directly to core client - supports strings, bytes, integers, and None
            let _as = aerospike_core::Key::new(namespace, set, key.into()).unwrap();
            Key { _as }
        }

        /// Fast-path Key constructor for the bench/per-op hot loop where
        /// the caller has an integer user_key. Skips the Python ``str(int)``
        /// conversion AND the ``PythonValue`` enum dispatch by going
        /// directly to ``Value::String(key.to_string())``. Matches the
        /// existing-cluster on-disk representation (string-typed user keys)
        /// — switch to ``from_int`` instead if you want int-typed user
        /// keys (a different on-server keyspace).
        ///
        /// Per-op cost drops from ~2 µs (PyO3 PythonValue dispatch +
        /// Python str()) to ~500 ns (positional PyO3 call + Rust string
        /// alloc).
        #[staticmethod]
        pub fn from_int_user_key(namespace: &str, set: &str, key: i64) -> Self {
            let value = aerospike_core::Value::String(key.to_string());
            let _as = aerospike_core::Key::new(namespace, set, value).unwrap();
            Key { _as }
        }

        #[staticmethod]
        /// Create a Key from a namespace, set, and digest (20-byte hash).
        /// The digest can be provided as bytes or a hex-encoded string.
        pub fn key_with_digest(namespace: &str, set: &str, digest: &Bound<'_, PyAny>) -> PyResult<Self> {
            let digest_bytes: Vec<u8> = if let Ok(bytes) = digest.extract::<Vec<u8>>() {
                bytes
            } else if let Ok(hex_str) = digest.extract::<String>() {
                hex::decode(&hex_str).map_err(|e| PyValueError::new_err(format!("Invalid hex digest: {}", e)))?
            } else if let Ok(byte_array) = digest.extract::<&[u8]>() {
                byte_array.to_vec()
            } else {
                return Err(PyTypeError::new_err("Digest must be bytes, bytearray, or hex string"));
            };

            if digest_bytes.len() != 20 {
                return Err(PyValueError::new_err(format!(
                    "Digest must be exactly 20 bytes, got {} bytes",
                    digest_bytes.len()
                )));
            }

            let mut digest_array = [0u8; 20];
            digest_array.copy_from_slice(&digest_bytes);

            let _as = aerospike_core::Key::with_digest(namespace, set, None, digest_array);

            Ok(Key { _as })
        }

        #[getter]
        pub fn get_namespace(&self) -> String {
            self._as.namespace().to_string()
        }

        #[getter]
        pub fn get_set_name(&self) -> String {
            self._as.set_name().to_string()
        }

        #[getter(value)]
        pub fn get_value(&self) -> Option<PythonValue> {
            // Return key value as-is (preserves integer, string, bytes, etc.)
            match self._as.user_key() {
                Some(v) => {
                    let pv: PythonValue = v.clone().into();
                    Some(pv)
                }
                None => None,
            }
        }

        #[getter]
        pub fn get_digest(&self) -> Option<String> {
            Some(hex::encode(self._as.digest()))
        }

        /// Partition (0-4095) that this key's digest maps to.
        #[getter]
        pub fn get_partition_id(&self) -> usize {
            self._as.partition_id()
        }

        fn __richcmp__(&self, other: PyRef<'_, Key>, op: CompareOp) -> bool {
            match op {
                CompareOp::Eq => self._as.digest() == other._as.digest(),
                CompareOp::Ne => self._as.digest() != other._as.digest(),
                _ => false,
            }
        }

        /// Hash of the digest, consistent with ``==``, so keys can be set
        /// members and dict keys.
        fn __hash__(&self) -> u64 {
            // The digest is already a uniformly distributed hash, so its
            // leading bytes serve directly without rehashing.
            let mut leading = [0u8; 8];
            leading.copy_from_slice(&self._as.digest()[..8]);
            u64::from_le_bytes(leading)
        }

        fn __str__(&self) -> PyResult<String> {
            Ok(format!("{}", self._as))
        }

        fn __repr__(&self) -> PyResult<String> {
            let s = self.__str__()?;
            Ok(format!("Key({})", s))
        }

        pub fn __copy__(&self) -> Self {
            self.clone()
        }

        pub fn __deepcopy__(&self, _memo: &Bound<PyDict>) -> Self {
            // fast bitwise copy instead of python's pickling process
            self.clone()
        }
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Blob
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, sequence, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Debug, Clone)]
    pub struct Blob {
        v: Vec<u8>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Blob {
        #[new]
        pub fn new(v: Vec<u8>) -> Self {
            Blob { v }
        }

        #[getter]
        pub fn get_value(&self) -> Vec<u8> {
            self.v.clone()
        }

        #[setter]
        pub fn set_value(&mut self, b: Vec<u8>) {
            self.v = b
        }

        /// Returns a string representation of the value.
        pub fn as_string(&self) -> String {
            PythonValue::Blob(self.v.clone()).as_string()
        }

        fn __getitem__(&mut self, idx: usize) -> PyResult<u8> {
            if idx >= self.v.len() {
                return Err(PyIndexError::new_err("index out of bounds"));
            }
            Ok(self.v[idx])
        }

        fn __setitem__(&mut self, idx: usize, v: u8) -> PyResult<()> {
            if idx >= self.v.len() {
                return Err(PyIndexError::new_err("index out of bounds"));
            }
            self.v[idx] = v;
            Ok(())
        }

        fn __hash__(&self) -> u64 {
            let mut s = DefaultHasher::new();
            self.v.hash(&mut s);
            s.finish()
        }

        fn __richcmp__<'a>(&self, other: &Bound<'a, PyAny>, op: CompareOp) -> bool {
            match op {
                CompareOp::Eq => {
                    let l: Result<Blob, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l.v;
                    }

                    let l: PyResult<Vec<u8>> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l;
                    }

                    false
                }
                CompareOp::Ne => {
                    let l: Result<Blob, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l.v;
                    }

                    let l: PyResult<Vec<u8>> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l;
                    }

                    true
                }
                _ => false,
            }
        }

        fn __add__(&self, other: &Bound<PyAny>) -> PyResult<Blob> {
            // Handle Blob + Blob
            if let Ok(other_blob) = other.extract::<Blob>() {
                let mut result = self.v.clone();
                result.extend_from_slice(&other_blob.v);
                return Ok(Blob::new(result));
            }

            // Handle Blob + Vec<u8>
            if let Ok(other_vec) = other.extract::<Vec<u8>>() {
                let mut result = self.v.clone();
                result.extend_from_slice(&other_vec);
                return Ok(Blob::new(result));
            }

            Err(PyTypeError::new_err("unsupported operand type(s) for +: 'Blob' and other type"))
        }

        fn __mul__(&self, other: &Bound<PyAny>) -> PyResult<Blob> {
            // Handle Blob * int
            if let Ok(count) = other.extract::<i32>() {
                if count < 0 {
                    return Err(PyValueError::new_err("can't multiply Blob by negative number"));
                }
                let mut result = Vec::new();
                for _ in 0..count {
                    result.extend_from_slice(&self.v);
                }
                return Ok(Blob::new(result));
            }

            Err(PyTypeError::new_err("unsupported operand type(s) for *: 'Blob' and other type"))
        }

        fn __iadd__(&mut self, other: &Bound<PyAny>) -> PyResult<()> {
            // Handle Blob += Blob
            if let Ok(other_blob) = other.extract::<Blob>() {
                self.v.extend_from_slice(&other_blob.v);
                return Ok(());
            }

            // Handle Blob += Vec<u8>
            if let Ok(other_vec) = other.extract::<Vec<u8>>() {
                self.v.extend_from_slice(&other_vec);
                return Ok(());
            }

            Err(PyTypeError::new_err("unsupported operand type(s) for +=: 'Blob' and other type"))
        }

        fn __imul__(&mut self, other: &Bound<PyAny>) -> PyResult<()> {
            // Handle Blob *= int
            if let Ok(count) = other.extract::<i32>() {
                if count < 0 {
                    return Err(PyValueError::new_err("can't multiply Blob by negative number"));
                }
                let original = self.v.clone();
                self.v.clear();
                for _ in 0..count {
                    self.v.extend_from_slice(&original);
                }
                return Ok(());
            }

            Err(PyTypeError::new_err("unsupported operand type(s) for *=: 'Blob' and other type"))
        }

        fn __delitem__(&mut self, idx: usize) -> PyResult<()> {
            if idx >= self.v.len() {
                return Err(PyIndexError::new_err("index out of bounds"));
            }
            self.v.remove(idx);
            Ok(())
        }

        fn __len__(&self) -> PyResult<usize> {
            Ok(self.v.len())
        }
    }

    impl fmt::Display for Blob {
        fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
            write!(f, "{}", self.as_string())
        }
    }

    // impl From<Blob> for PythonValue {
    //     fn from(input: Blob) -> Self {
    //         PythonValue::Blob(input.v.clone())
    //     }
    // }

    // impl Into<PythonValue> for Blob {
    //     fn into(self) -> PythonValue {
    //         PythonValue::Blob(self.v)
    //     }
    // }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Map
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// A map to be stored key-ordered (``MapOrder.KEY_ORDERED``).
    ///
    /// A plain ``dict`` is written as an unordered map. The server stores the
    /// entries sorted either way, so a read cannot tell the two apart by
    /// looking at them -- but it will not binary-search a map that was not
    /// *declared* ordered, so keyed and range access on one fall back to a
    /// scan. Wrapping the dict declares the order, and the client packs the
    /// entries sorted, which the server requires: it rejects a map that claims
    /// ``K_ORDERED`` and arrives out of order.
    ///
    /// The effect is durable, not confined to the write. The order flag is
    /// stored with the record and survives later modification, so it governs
    /// the cost of every subsequent access, by any client, until the map is
    /// rewritten unordered.
    ///
    /// Reads of a key-ordered map return this type, so the round trip is
    /// symmetric. It subclasses ``dict``, so it behaves as one everywhere.
    ///
    /// Example::
    ///
    ///     await client.put(key, {"scores": SortedMap({"zoe": 3, "amy": 1})})
    ///
    ///     scores = (await client.get(key)).bins["scores"]
    ///     scores["amy"]                   # 1
    ///     scores == {"amy": 1, "zoe": 3}  # True
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(extends = PyDict, module = "_native")]
    #[derive(Debug)]
    pub struct SortedMap;

    #[pymethods]
    impl SortedMap {
        #[new]
        #[pyo3(signature = (*_args, **_kwargs))]
        fn __new__(_args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) -> Self {
            SortedMap
        }

        #[pyo3(signature = (*args, **kwargs))]
        fn __init__(
            slf: &Bound<'_, Self>,
            args: &Bound<'_, PyTuple>,
            kwargs: Option<&Bound<'_, PyDict>>,
        ) -> PyResult<()> {
            // `super(SortedMap, self)`, not `super(dict, self)` -- the latter
            // resolves past dict to object, whose __init__ takes no arguments.
            PySuper::new(&Self::type_object(slf.py()), slf)?
                .call_method("__init__", args.to_owned(), kwargs)?;
            Ok(())
        }

        fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
            // Built from the items: this *is* the dict, so delegating to
            // `.repr()` would dispatch straight back here.
            let as_dict: &Bound<'_, PyDict> = slf.as_any().cast()?;
            let mut parts: Vec<String> = Vec::with_capacity(as_dict.len());
            for (k, v) in as_dict.iter() {
                parts.push(format!("{}: {}", k.repr()?, v.repr()?));
            }
            Ok(format!("SortedMap({{{}}})", parts.join(", ")))
        }
    }

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, sequence, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Debug, Clone)]
    pub struct Map {
        v: HashMap<PythonValue, PythonValue>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Map {
        #[new]
        pub fn new(v: HashMap<PythonValue, PythonValue>) -> Self {
            Map { v }
        }

        #[getter]
        pub fn get_value(&self) -> HashMap<PythonValue, PythonValue> {
            self.v.clone()
        }

        #[setter]
        pub fn set_value(&mut self, b: HashMap<PythonValue, PythonValue>) {
            self.v = b
        }

        /// Returns a string representation of the value.
        pub fn as_string(&self) -> String {
            PythonValue::HashMap(self.v.clone()).as_string()
        }

        // TODO: Change HashMap into BTreeMap and use that
        // This requires Rust Client implementation first
        // fn __hash__(&self) -> u64 {
        //     let mut s = DefaultHasher::new();
        //     self.v.hash(&mut s);
        //     s.finish()
        // }

        fn __richcmp__<'a>(&self, other: &Bound<'a, PyAny>, op: CompareOp) -> bool {
            match op {
                CompareOp::Eq => {
                    let l: Result<Map, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l.v;
                    }

                    let l: PyResult<HashMap<PythonValue, PythonValue>> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l;
                    }

                    false
                }
                CompareOp::Ne => {
                    let l: Result<Map, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l.v;
                    }

                    let l: PyResult<HashMap<PythonValue, PythonValue>> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l;
                    }

                    true
                }
                _ => false,
            }
        }

        fn __str__(&self) -> PyResult<String> {
            // Convert HashMap to JSON-like string format
            let mut items = Vec::new();
            for (k, v) in &self.v {
                let key_str = match k {
                    PythonValue::String(s) => format!("\"{}\"", s),
                    _ => format!("{:?}", k),
                };
                let val_str = match v {
                    PythonValue::String(s) => format!("\"{}\"", s),
                    PythonValue::Int(i) => i.to_string(),
                    PythonValue::Bool(b) => b.to_string(),
                    PythonValue::Float(f) => f.to_string(),
                    PythonValue::Nil => "None".to_string(),
                    _ => format!("{:?}", v),
                };
                items.push(format!("{}: {}", key_str, val_str));
            }
            Ok(format!("{{{}}}", items.join(", ")))
        }

        fn __repr__(&self) -> PyResult<String> {
            let s = self.__str__()?;
            Ok(format!("Map({})", s))
        }
    }

    impl fmt::Display for Map {
        fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
            write!(f, "{}", self.as_string())
        }
    }

    // impl From<HashMap> for PythonValue {
    //     fn from(input: HashMap) -> Self {
    //         PythonValue::HashMap(input.v.clone())
    //     }
    // }

    // impl Into<PythonValue> for HashMap {
    //     fn into(self) -> PythonValue {
    //         PythonValue::HashMap(self.v)
    //     }
    // }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  List
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    fn format_python_value(value: &PythonValue) -> String {
        match value {
            PythonValue::String(s) => format!("\"{}\"", s),
            PythonValue::Int(i) => i.to_string(),
            PythonValue::Bool(b) => if *b { "True".to_string() } else { "False".to_string() },
            PythonValue::Float(f) => f.to_string(),
            PythonValue::Nil => "None".to_string(),
            PythonValue::List(l) => {
                let mut items = Vec::new();
                for item in l {
                    items.push(format_python_value(item));
                }
                format!("[{}]", items.join(", "))
            },
            PythonValue::HashMap(h) => {
                let mut items = Vec::new();
                let mut sorted_entries: Vec<_> = h.iter().collect();
                sorted_entries.sort_by_key(|(k, _)| format_python_value(k));
                for (k, v) in sorted_entries {
                    items.push(format!("{}: {}", format_python_value(k), format_python_value(v)));
                }
                format!("{{{}}}", items.join(", "))
            },
            PythonValue::OrderedMap(pairs) => {
                let items: Vec<_> = pairs.iter()
                    .map(|(k, v)| format!("{}: {}", format_python_value(k), format_python_value(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            },
            _ => format!("{:?}", value),
        }
    }

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, sequence, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Debug, Clone)]
    pub struct List {
        v: Vec<PythonValue>,
        index: usize,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl List {
        #[new]
        pub fn new(v: Vec<PythonValue>) -> Self {
            List { v, index: 0 }
        }

        #[getter]
        pub fn get_value(&self) -> Vec<PythonValue> {
            self.v.clone()
        }

        #[setter]
        pub fn set_value(&mut self, geo: Vec<PythonValue>) {
            self.v = geo
        }

        /// Returns a string representation of the value.
        pub fn as_string(&self) -> String {
            PythonValue::List(self.v.clone()).as_string()
        }

        fn __str__(&self) -> PyResult<String> {
            // Convert internal representation to Python list format
            let mut items = Vec::new();
            for item in &self.v {
                let item_str = format_python_value(item);
                items.push(item_str);
            }
            Ok(format!("[{}]", items.join(", ")))
        }

        fn __repr__(&self) -> PyResult<String> {
            let s = self.__str__()?;
            Ok(format!("List({})", s))
        }

        fn __getitem__(&mut self, idx: usize) -> PyResult<PythonValue> {
            if idx >= self.v.len() {
                return Err(PyIndexError::new_err("index out of bounds"));
            }
            Ok(self.v[idx].clone())
        }

        fn __setitem__(&mut self, idx: usize, v: PythonValue) -> PyResult<()> {
            if idx >= self.v.len() {
                return Err(PyIndexError::new_err("index out of bounds"));
            }
            self.v[idx] = v;
            Ok(())
        }

        fn __delitem__(&mut self, idx: usize) -> PyResult<()> {
            if idx >= self.v.len() {
                return Err(PyIndexError::new_err("index out of bounds"))
            }
            self.v.remove(idx);
            Ok(())
        }

        fn __concat__(&self, mut other: List) -> PyResult<List> {
            let mut new_list = self.v.clone();
            new_list.append(&mut other.v);
            Ok(List { v: new_list, index: 0 })
        }

        fn __inplace_concat__(&mut self, mut other: List) -> PyResult<List> {
            self.v.append(&mut other.v);
            Ok(self.clone())
        }

        fn __repeat__(&self, times: usize) -> PyResult<List> {
            let og = self.v.clone();
            let len = self.v.len();
            let new_list: Vec<_> = og.into_iter().cycle().take(len * times).collect();
            Ok(List { v: new_list, index: 0 })
        }

        fn __inplace_repeat__(&mut self, times: usize) -> PyResult<List> {
            self.__repeat__(times)
        }
        fn __hash__(&self) -> u64 {
            let mut s = DefaultHasher::new();
            self.v.hash(&mut s);
            s.finish()
        }

        fn __len__(&self) -> usize {
            self.v.len()
        }
        fn __richcmp__<'a>(&self, other: &Bound<'a, PyAny>, op: CompareOp) -> bool {
            match op {
                CompareOp::Eq => {
                    let l: Result<List, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l.v;
                    }

                    let l: PyResult<Vec<PythonValue>> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l;
                    }

                    false
                }
                CompareOp::Ne => {
                    let l: Result<List, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l.v;
                    }

                    let l: PyResult<Vec<PythonValue>> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l;
                    }

                    true
                }
                _ => false,
            }
        }

        fn __iter__(&self) -> Self {
            self.clone()
        }

        fn __next__<'a>(&mut self, py: Python<'a>) -> Option<Py<PyAny>> {
            let res = self.v.get(self.index);
            self.index += 1;
            res.map(|v| v.clone().into_pyobject(py).unwrap().unbind())
        }
    }

    impl fmt::Display for List {
        fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
            write!(f, "{}", self.as_string())
        }
    }

    // impl From<List> for PythonValue {
    //     fn from(input: List) -> Self {
    //         PythonValue::List(input.v.clone())
    //     }
    // }

    // impl Into<PythonValue> for List {
    //     fn into(self) -> PythonValue {
    //         PythonValue::List(self.v)
    //     }
    // }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  GeoJSON
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    // Resolved once per process: dict construction runs per value on geo
    // workloads, and a per-call import convoys free-threaded builds on the
    // `sys.modules` lock.
    static JSON_DUMPS: pyo3::sync::PyOnceLock<Py<PyAny>> =
        pyo3::sync::PyOnceLock::new();

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Debug, Clone)]
    pub struct GeoJSON {
        v: String,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl GeoJSON {
        #[new]
        pub fn new<'a>(py: Python<'a>, v: &Bound<'a, PyAny>) -> PyResult<Self> {
            // Accept both String and dict inputs
            if let Ok(s) = v.extract::<String>() {
                return Ok(GeoJSON { v: s });
            }

            // If it's already a GeoJSON object, extract its value
            if let Ok(geo) = v.extract::<GeoJSON>() {
                return Ok(geo);
            }

            // Try to extract as dict and serialize to JSON
            if let Ok(dict) = v.cast::<PyDict>() {
                let json_dumps = JSON_DUMPS.import(py, "json", "dumps")?;
                let json_string: String = json_dumps.call1((dict,))?.extract()?;
                return Ok(GeoJSON { v: json_string });
            }


            Err(PyTypeError::new_err(
                "GeoJSON constructor requires a string, dict, or GeoJSON object"
            ))
        }

        #[getter]
        pub fn get_value(&self) -> String {
            self.v.clone()
        }

        #[setter]
        pub fn set_value(&mut self, geo: String) {
            self.v = geo
        }

        /// Returns a string representation of the value.
        pub fn as_string(&self) -> String {
            PythonValue::GeoJSON(self.v.clone()).as_string()
        }

        fn __richcmp__<'a>(&self, other: &Bound<'a, PyAny>, op: CompareOp) -> bool {
            match op {
                CompareOp::Eq => {
                    let l: Result<GeoJSON, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l.v;
                    }

                    let l: PyResult<String> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l;
                    }

                    false
                }
                CompareOp::Ne => {
                    let l: Result<GeoJSON, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l.v;
                    }

                    let l: PyResult<String> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l;
                    }

                    true
                }
                _ => false,
            }
        }

        fn __str__(&self) -> PyResult<String> {
            Ok(self.v.clone())
        }

        fn __repr__(&self) -> PyResult<String> {
            Ok(format!("GeoJSON({})", self.v))
        }
    }

    impl fmt::Display for GeoJSON {
        fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
            write!(f, "{}", self.as_string())
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  HLL
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, sequence, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Debug, Clone)]
    pub struct HLL {
        v: Vec<u8>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl HLL {
        #[new]
        pub fn new(v: Vec<u8>) -> Self {
            HLL { v }
        }

        #[getter]
        pub fn get_value(&self) -> Vec<u8> {
            self.v.clone()
        }

        #[setter]
        pub fn set_value(&mut self, hll: Vec<u8>) {
            self.v = hll
        }

        /// Returns a string representation of the value.
        pub fn as_string(&self) -> String {
            PythonValue::HLL(self.v.clone()).as_string()
        }

        fn __richcmp__<'a>(&self, other: &Bound<'a, PyAny>, op: CompareOp) -> bool {
            match op {
                CompareOp::Eq => {
                    let l: Result<HLL, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l.v;
                    }

                    let l: PyResult<Vec<u8>> = other.extract();
                    if let Ok(l) = l {
                        return self.v == l;
                    }

                    false
                }
                CompareOp::Ne => {
                    let l: Result<HLL, _> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l.v;
                    }

                    let l: PyResult<Vec<u8>> = other.extract();
                    if let Ok(l) = l {
                        return self.v != l;
                    }

                    true
                }
                _ => false,
            }
        }
    }

    impl fmt::Display for HLL {
        fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
            write!(f, "{}", self.as_string())
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Vector
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Element type of a :class:`Vector`: how each element is stored on the wire.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum VectorElementType {
        /// IEEE 754 half precision.
        #[pyo3(name = "FLOAT16")]
        Float16,
        /// 32-bit signed integer.
        #[pyo3(name = "INT32")]
        Int32,
        /// 32-bit IEEE 754 float.
        #[pyo3(name = "FLOAT32")]
        Float32,
        /// 64-bit IEEE 754 double.
        #[pyo3(name = "FLOAT64")]
        Float64,
    }

    #[pymethods]
    impl VectorElementType {
        fn __richcmp__(&self, other: &VectorElementType, op: CompareOp) -> PyResult<bool> {
            match op {
                CompareOp::Eq => Ok(self == other),
                CompareOp::Ne => Ok(self != other),
                _ => Err(pyo3::exceptions::PyNotImplementedError::new_err("Only == and != comparisons are supported")),
            }
        }

        fn __hash__(&self) -> u64 {
            let mut hasher = DefaultHasher::new();
            self.hash(&mut hasher);
            hasher.finish()
        }
    }

    impl VectorElementType {
        const fn name(self) -> &'static str {
            match self {
                VectorElementType::Float16 => "FLOAT16",
                VectorElementType::Int32 => "INT32",
                VectorElementType::Float32 => "FLOAT32",
                VectorElementType::Float64 => "FLOAT64",
            }
        }

        const fn item_size(self) -> usize {
            match self {
                VectorElementType::Float16 => 2,
                VectorElementType::Int32 | VectorElementType::Float32 => 4,
                VectorElementType::Float64 => 8,
            }
        }
    }

    impl From<aerospike_core::VectorElementType> for VectorElementType {
        fn from(input: aerospike_core::VectorElementType) -> Self {
            match input {
                aerospike_core::VectorElementType::Float16 => VectorElementType::Float16,
                aerospike_core::VectorElementType::Int32 => VectorElementType::Int32,
                aerospike_core::VectorElementType::Float32 => VectorElementType::Float32,
                aerospike_core::VectorElementType::Float64 => VectorElementType::Float64,
            }
        }
    }

    /// Compares elements the way Python compares numbers, so `-0.0 == 0.0`.
    /// Vectors hold no NaN, which keeps this reflexive.
    fn vectors_equal(a: &aerospike_core::Vector, b: &aerospike_core::Vector) -> bool {
        match (a.data(), b.data()) {
            (VectorData::Float16(x), VectorData::Float16(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(&p, &q)| p == q || (p | q) & F16_MAGNITUDE == 0)
            }
            (VectorData::Int32(x), VectorData::Int32(y)) => x == y,
            (VectorData::Float32(x), VectorData::Float32(y)) => x == y,
            (VectorData::Float64(x), VectorData::Float64(y)) => x == y,
            _ => false,
        }
    }

    const F16_MAGNITUDE: u16 = 0x7fff;

    /// Hashes a vector consistently with [`vectors_equal`]: both zeros hash as `+0.0`.
    fn hash_vector<H: Hasher>(v: &aerospike_core::Vector, state: &mut H) {
        v.element_type().code().hash(state);
        match v.data() {
            VectorData::Float16(d) => d
                .iter()
                .for_each(|&b| (if b & F16_MAGNITUDE == 0 { 0 } else { b }).hash(state)),
            VectorData::Int32(d) => d.hash(state),
            VectorData::Float32(d) => d
                .iter()
                .for_each(|&x| (if x == 0.0 { 0 } else { x.to_bits() }).hash(state)),
            VectorData::Float64(d) => d
                .iter()
                .for_each(|&x| (if x == 0.0 { 0 } else { x.to_bits() }).hash(state)),
        }
    }

    /// Rounds each element to the nearest `float16`, refusing a finite value
    /// that has no finite `float16` form rather than storing an infinity.
    fn float16_bits(data: Vec<f64>) -> PyResult<Vec<u16>> {
        data.into_iter()
            .enumerate()
            .map(|(i, x)| {
                let h = half::f16::from_f64(x);
                if x.is_finite() && !h.is_finite() {
                    return Err(RustClientError(aerospike_core::Error::invalid_argument(format!(
                        "vector element {i} is out of float16 range: {x}"
                    )))
                    .into());
                }
                Ok(h.to_bits())
            })
            .collect()
    }

    fn vector_from_sequence(
        data: &Bound<'_, PyAny>,
        element_type: VectorElementType,
    ) -> PyResult<aerospike_core::Vector> {
        let core = match element_type {
            VectorElementType::Float16 => aerospike_core::Vector::try_float16(float16_bits(data.extract()?)?),
            VectorElementType::Int32 => aerospike_core::Vector::try_int32(data.extract()?),
            VectorElementType::Float32 => aerospike_core::Vector::try_float32(data.extract()?),
            VectorElementType::Float64 => aerospike_core::Vector::try_float64(data.extract()?),
        };
        Ok(core.map_err(RustClientError)?)
    }

    fn has_buffer(data: &Bound<'_, PyAny>) -> bool {
        // SAFETY: `data` is a live object; the call only inspects its type slots.
        unsafe { ffi::PyObject_CheckBuffer(data.as_ptr()) == 1 }
    }

    const fn is_native_byte_order(order: u8) -> bool {
        matches!(order, b'@' | b'=')
            || if cfg!(target_endian = "little") {
                order == b'<'
            } else {
                matches!(order, b'>' | b'!')
            }
    }

    /// The element type a buffer format describes. A byte-swapped format is
    /// refused: reading it in place would decode every element wrong.
    fn buffer_element_type(format: &CStr) -> Option<VectorElementType> {
        let native = match format.to_bytes() {
            [_] => true,
            [order, _] => is_native_byte_order(*order),
            _ => false,
        };
        if !native {
            return None;
        }
        match ElementType::from_format(format) {
            ElementType::Float { bytes: 2 } => Some(VectorElementType::Float16),
            ElementType::SignedInteger { bytes: 4 } => Some(VectorElementType::Int32),
            ElementType::Float { bytes: 4 } => Some(VectorElementType::Float32),
            ElementType::Float { bytes: 8 } => Some(VectorElementType::Float64),
            _ => None,
        }
    }

    /// Copies a 1-D buffer's elements, following its stride.
    fn buffer_elements<T: Copy>(buf: &PyUntypedBuffer) -> Vec<T> {
        let count = buf.item_count();
        let base = buf.buf_ptr().cast::<u8>().cast_const();
        let stride = buf.strides()[0];
        let size = std::mem::size_of::<T>();
        // SAFETY: while `buf` holds the buffer, its exporter guarantees `count`
        // items of `size` bytes at `base + i * stride`; the caller checked the
        // item size, and unaligned reads place no alignment demand on `base`.
        unsafe {
            if stride == size as isize {
                let mut out = Vec::<T>::with_capacity(count);
                std::ptr::copy_nonoverlapping(base, out.as_mut_ptr().cast::<u8>(), count * size);
                out.set_len(count);
                out
            } else {
                (0..count)
                    .map(|i| base.offset(i as isize * stride).cast::<T>().read_unaligned())
                    .collect()
            }
        }
    }

    /// Builds a vector from a 1-D buffer such as a numpy array, ``array.array``
    /// or ``memoryview``. The buffer's format selects the element type.
    fn vector_from_buffer(
        data: &Bound<'_, PyAny>,
        element_type: Option<VectorElementType>,
    ) -> PyResult<aerospike_core::Vector> {
        // PyO3 refuses an exporter that leaves `strides` null, which the buffer
        // protocol allows for contiguous data (ctypes arrays do it); a memoryview
        // re-exports the same memory with strides filled in.
        let buf = PyUntypedBuffer::get(data)
            .or_else(|_| PyUntypedBuffer::get(PyMemoryView::from(data)?.as_any()))?;
        if buf.dimensions() != 1 {
            return Err(RustClientError(aerospike_core::Error::invalid_argument(format!(
                "Vector requires a 1-D buffer, got {}-D",
                buf.dimensions()
            )))
            .into());
        }
        if buf.suboffsets().is_some_and(|s| s[0] >= 0) {
            return Err(PyTypeError::new_err("Vector does not read indirect (suboffset) buffers"));
        }
        let format = buf.format();
        let detected = buffer_element_type(format)
            .filter(|t| t.item_size() == buf.item_size())
            .ok_or_else(|| {
                PyTypeError::new_err(format!(
                    "unsupported buffer format '{}' for Vector; use float16, int32, float32 or \
                     float64 in native byte order",
                    format.to_string_lossy()
                ))
            })?;
        if let Some(requested) = element_type {
            if requested != detected {
                return Err(PyTypeError::new_err(format!(
                    "element_type {} does not match buffer format '{}'",
                    requested.name(),
                    format.to_string_lossy()
                )));
            }
        }

        let core = match detected {
            VectorElementType::Float16 => aerospike_core::Vector::try_float16(buffer_elements(&buf)),
            VectorElementType::Int32 => aerospike_core::Vector::try_int32(buffer_elements(&buf)),
            VectorElementType::Float32 => aerospike_core::Vector::try_float32(buffer_elements(&buf)),
            VectorElementType::Float64 => aerospike_core::Vector::try_float64(buffer_elements(&buf)),
        };
        Ok(core.map_err(RustClientError)?)
    }

    /// A dense numeric vector, stored in a bin as a ``VECTOR`` value and compared
    /// with :meth:`FilterExpression.vector_distance`.
    ///
    /// Build one from a sequence of numbers, with an optional element type
    /// (default :attr:`VectorElementType.FLOAT32`), or from a 1-D buffer such as a
    /// numpy array, ``array.array`` or ``memoryview``, whose format selects the
    /// element type::
    ///
    ///     embedding = Vector([0.12, 0.98, -0.34])
    ///     counts = Vector([3, 0, 7], VectorElementType.INT32)
    ///     compact = Vector(np.array([0.5, 1.5], dtype=np.float16))
    ///
    /// A vector has at least one element, and every floating-point element is
    /// finite: an empty sequence, a NaN or an infinity raises :class:`ValueError`.
    /// ``FLOAT16`` elements given as Python floats are rounded to the nearest
    /// half-precision value.
    ///
    /// A vector is an immutable, hashable sequence: ``len(v)``, ``v[i]``, slicing
    /// and iteration work as they do for a tuple, and two vectors are equal when
    /// their element types and elements are equal. It also exposes its elements
    /// as a read-only buffer, so ``numpy.asarray(v)`` and ``memoryview(v)`` read
    /// them in place without copying.
    ///
    /// Writing a vector, or sending a vector expression, requires every cluster
    /// node to run Aerospike Server 8.2.1 or later; otherwise the client refuses
    /// the command before sending it. The server stores a vector only as a whole
    /// bin value, not nested in a list or map bin.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, frozen, sequence, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Debug, Clone)]
    pub struct Vector {
        pub(crate) v: aerospike_core::Vector,
    }

    impl Vector {
        fn element<'py>(&self, py: Python<'py>, index: usize) -> PyResult<Bound<'py, PyAny>> {
            match self.v.data() {
                VectorData::Float16(d) => half::f16::from_bits(d[index]).to_f64().into_bound_py_any(py),
                VectorData::Int32(d) => d[index].into_bound_py_any(py),
                VectorData::Float32(d) => d[index].into_bound_py_any(py),
                VectorData::Float64(d) => d[index].into_bound_py_any(py),
            }
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Vector {
        #[new]
        #[pyo3(signature = (data, element_type = None))]
        pub fn new(
            #[gen_stub(override_type(
                type_repr = "_native.Vector | typing.Sequence[builtins.float] | collections.abc.Buffer",
                imports = ("typing", "builtins", "collections.abc")
            ))]
            data: &Bound<'_, PyAny>,
            element_type: Option<VectorElementType>,
        ) -> PyResult<Self> {
            if let Ok(existing) = data.cast::<Vector>() {
                let existing = existing.get();
                if element_type.is_some_and(|t| t != existing.v.element_type().into()) {
                    return Err(PyTypeError::new_err(
                        "element_type does not match the Vector's element type",
                    ));
                }
                return Ok(existing.clone());
            }
            // Lists and tuples are the common input and never export a buffer.
            if !data.is_instance_of::<PyList>() && !data.is_instance_of::<PyTuple>() && has_buffer(data) {
                return Ok(Vector { v: vector_from_buffer(data, element_type)? });
            }
            let v = vector_from_sequence(data, element_type.unwrap_or(VectorElementType::Float32))?;
            Ok(Vector { v })
        }

        /// Decodes a complete vector wire value: a version byte, an element-type
        /// byte, the dimension count as a little-endian ``u32``, then the
        /// little-endian elements. Use it for a vector carried as ``bytes``, such
        /// as one an application stored in a blob bin.
        ///
        /// Example::
        ///
        ///     record = await client.get(key, ["embedding_blob"])
        ///     embedding = Vector.from_wire_bytes(record.bins["embedding_blob"])
        ///
        /// Args:
        ///     data: The encoded vector.
        ///
        /// Returns:
        ///     The decoded vector.
        ///
        /// Raises:
        ///     ValueError: The bytes are not a valid vector wire value.
        #[staticmethod]
        pub fn from_wire_bytes(
            #[gen_stub(override_type(type_repr = "builtins.bytes", imports = ("builtins")))] data: Vec<u8>,
        ) -> PyResult<Self> {
            // Core reports a malformed vector as a bad server response; these
            // bytes come from the caller, so the error is the argument's.
            let v = aerospike_core::Vector::from_wire_bytes(&data).map_err(|e| {
                RustClientError(aerospike_core::Error::invalid_argument(format!(
                    "malformed vector wire bytes: {e}"
                )))
            })?;
            Ok(Vector { v })
        }

        /// The element type.
        #[getter]
        pub fn element_type(&self) -> VectorElementType {
            self.v.element_type().into()
        }

        /// The number of elements.
        #[getter]
        pub fn dimensions(&self) -> usize {
            self.v.dimensions()
        }

        /// The elements as a new list of ``int`` (``INT32``) or ``float`` values.
        #[gen_stub(override_return_type(
            type_repr = "builtins.list[builtins.int] | builtins.list[builtins.float]",
            imports = ("builtins")
        ))]
        #[getter]
        pub fn value<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            match self.v.data() {
                VectorData::Float16(d) => d
                    .iter()
                    .map(|&bits| half::f16::from_bits(bits).to_f64())
                    .collect::<Vec<f64>>()
                    .into_bound_py_any(py),
                VectorData::Int32(d) => d.into_bound_py_any(py),
                VectorData::Float32(d) => d.into_bound_py_any(py),
                VectorData::Float64(d) => d.into_bound_py_any(py),
            }
        }

        fn __len__(&self) -> usize {
            self.v.dimensions()
        }

        #[gen_stub(override_return_type(
            type_repr = "builtins.int | builtins.float | builtins.list[builtins.int] | builtins.list[builtins.float]",
            imports = ("builtins")
        ))]
        fn __getitem__<'py>(&self, py: Python<'py>, index: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            let len = self.v.dimensions() as isize;
            if let Ok(slice) = index.cast::<PySlice>() {
                let bounds = slice.indices(len)?;
                let elements = (0..bounds.slicelength as isize)
                    .map(|i| self.element(py, (bounds.start + i * bounds.step) as usize))
                    .collect::<PyResult<Vec<_>>>()?;
                return PyList::new(py, elements).map(Bound::into_any);
            }
            let i: isize = index.extract()?;
            let position = if i < 0 { i + len } else { i };
            if !(0..len).contains(&position) {
                return Err(PyIndexError::new_err("Vector index out of range"));
            }
            self.element(py, position as usize)
        }

        #[gen_stub(override_return_type(
            type_repr = "typing.Iterator[builtins.int | builtins.float]",
            imports = ("typing", "builtins")
        ))]
        fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
            self.value(py)?.try_iter()
        }

        /// Exposes the elements as a read-only buffer in native byte order, so
        /// ``memoryview(v)`` and ``numpy.asarray(v)`` read them without copying.
        #[gen_stub(skip)]
        unsafe fn __getbuffer__(slf: Bound<'_, Self>, view: *mut ffi::Py_buffer, flags: c_int) -> PyResult<()> {
            if view.is_null() {
                return Err(PyBufferError::new_err("buffer view is null"));
            }
            if flags & ffi::PyBUF_WRITABLE == ffi::PyBUF_WRITABLE {
                return Err(PyBufferError::new_err("Vector is read-only"));
            }
            let v = &slf.get().v;
            let (data, item_size, format): (*const c_void, usize, &'static CStr) = match v.data() {
                VectorData::Float16(d) => (d.as_ptr().cast(), 2, c"e"),
                VectorData::Int32(d) => (d.as_ptr().cast(), 4, c"i"),
                VectorData::Float32(d) => (d.as_ptr().cast(), 4, c"f"),
                VectorData::Float64(d) => (d.as_ptr().cast(), 8, c"d"),
            };
            let count = v.dimensions();
            // SAFETY: `view` is non-null and owned by the caller. The elements stay
            // valid and unchanged for the view's lifetime: the Vector is immutable
            // and `obj` keeps it alive. The shape box is freed in `__releasebuffer__`.
            unsafe {
                let shape = Box::into_raw(Box::new(count as ffi::Py_ssize_t));
                (*view).buf = data.cast_mut();
                (*view).len = (count * item_size) as ffi::Py_ssize_t;
                (*view).readonly = 1;
                (*view).itemsize = item_size as ffi::Py_ssize_t;
                (*view).format = if flags & ffi::PyBUF_FORMAT == ffi::PyBUF_FORMAT {
                    format.as_ptr().cast_mut()
                } else {
                    std::ptr::null_mut()
                };
                (*view).ndim = 1;
                (*view).shape = if flags & ffi::PyBUF_ND == ffi::PyBUF_ND { shape } else { std::ptr::null_mut() };
                (*view).strides = if flags & ffi::PyBUF_STRIDES == ffi::PyBUF_STRIDES {
                    &mut (*view).itemsize
                } else {
                    std::ptr::null_mut()
                };
                (*view).suboffsets = std::ptr::null_mut();
                (*view).internal = shape.cast();
                (*view).obj = slf.into_any().into_ptr();
            }
            Ok(())
        }

        #[gen_stub(skip)]
        unsafe fn __releasebuffer__(&self, view: *mut ffi::Py_buffer) {
            // SAFETY: `internal` holds the shape box `__getbuffer__` allocated for this view.
            drop(unsafe { Box::from_raw((*view).internal.cast::<ffi::Py_ssize_t>()) });
        }

        fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> bool {
            let equal = other.cast::<Vector>().is_ok_and(|other| vectors_equal(&self.v, &other.get().v));
            match op {
                CompareOp::Eq => equal,
                CompareOp::Ne => !equal,
                _ => false,
            }
        }

        fn __hash__(&self) -> u64 {
            let mut hasher = DefaultHasher::new();
            hash_vector(&self.v, &mut hasher);
            hasher.finish()
        }

        fn __repr__(&self) -> String {
            let elements = match self.v.data() {
                VectorData::Float16(d) => format!(
                    "{:?}",
                    d.iter().map(|&b| half::f16::from_bits(b).to_f64()).collect::<Vec<_>>()
                ),
                VectorData::Int32(d) => format!("{d:?}"),
                VectorData::Float32(d) => format!("{d:?}"),
                VectorData::Float64(d) => format!("{d:?}"),
            };
            let element_type: VectorElementType = self.v.element_type().into();
            format!("Vector({elements}, VectorElementType.{})", element_type.name())
        }
    }

    // The buffer slots take raw `Py_buffer` pointers, which the stub generator
    // cannot type; these are the Python-level methods CPython derives from them.
    pyo3_stub_gen::inventory::submit! {
        pyo3_stub_gen::derive::gen_methods_from_python! {
            r#"
            class Vector:
                def __buffer__(self, flags: int) -> memoryview: ...
                def __release_buffer__(self, buffer: memoryview) -> None: ...
            "#
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  SpecialValue (CDT range / value boundaries)
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Markers for Aerospike CDT map/list range and value operations.
    /// These use dedicated wire particles, distinct from Python ``float('inf')`` or the ``"*"`` string.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, name = "SpecialValue", module = "_native")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum SpecialValue {
        /// Null particle boundary (e.g. unbounded start of a key range).
        #[pyo3(name = "NULL")]
        Null,
        /// Positive infinity particle (e.g. unbounded end of a value range).
        #[pyo3(name = "INFINITY")]
        Infinity,
        /// Wildcard particle for value matching.
        #[pyo3(name = "WILDCARD")]
        Wildcard,
    }

    #[pymethods]
    impl SpecialValue {
        fn __richcmp__(
            &self,
            other: &SpecialValue,
            op: pyo3::class::basic::CompareOp,
        ) -> pyo3::PyResult<bool> {
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
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  PythonValue
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    // Container for bin values stored in the Aerospike database.
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[allow(clippy::upper_case_acronyms)]
    pub enum PythonValue {
        /// Empty value.
        Nil,
        /// Boolean value.
        Bool(bool),
        /// Integer value. All integers are represented as 64-bit numerics in Aerospike.
        Int(i64),
        /// Floating point value. All floating point values are stored in 64-bit IEEE-754 format in
        /// Aerospike. Aerospike server v3.6.0 and later support double data type.
        Float(ordered_float::OrderedFloat<f64>),
        /// String value.
        String(String),
        /// Byte array value.
        Blob(Vec<u8>),
        /// List data type is an ordered collection of values. Lists can contain values of any
        /// supported data type. List data order is maintained on writes and reads.
        List(Vec<PythonValue>),
        /// Map data type is a collection of key-value pairs. Each key can only appear once in a
        /// collection and is associated with a value. Map keys and values can be any supported data
        /// type.
        HashMap(HashMap<PythonValue, PythonValue>),
        /// Ordered map preserving key order from the server (K-ordered / KV-ordered maps).
        /// Stored as Vec of pairs so insertion order into PyDict yields sorted keys.
        OrderedMap(Vec<(PythonValue, PythonValue)>),
        /// Map the caller asked to be stored key-ordered, via the `SortedMap` wrapper.
        /// Carries the same pairs as `HashMap`; the variant is the request, and the
        /// key order is produced at the boundary when this becomes a core
        /// `Value::SortedMap`. Never produced by decoding a server response.
        SortedMap(Vec<(PythonValue, PythonValue)>),
        /// GeoJSON data type are JSON formatted strings to encode geospatial information.
        GeoJSON(String),

        /// HLL value
        HLL(Vec<u8>),

        /// Dense numeric vector (see :class:`Vector`).
        Vector(aerospike_core::Vector),

        /// CDT boundary markers (see :class:`SpecialValue`).
        CdtSpecial(SpecialValue),
    }

    #[allow(clippy::derived_hash_with_manual_eq)]
    impl Hash for PythonValue {
        fn hash<H: Hasher>(&self, state: &mut H) {
            match *self {
                PythonValue::Nil => {
                    let v: Option<u8> = None;
                    v.hash(state);
                }
                PythonValue::Bool(ref val) => val.hash(state),
                PythonValue::Int(ref val) => val.hash(state),
                PythonValue::Float(ref val) => val.hash(state),
                PythonValue::String(ref val) | PythonValue::GeoJSON(ref val) => val.hash(state),
                PythonValue::Blob(ref val) | PythonValue::HLL(ref val) => val.hash(state),
                PythonValue::List(ref val) => val.hash(state),
                PythonValue::Vector(ref val) => hash_vector(val, state),
                PythonValue::HashMap(_)
                | PythonValue::OrderedMap(_)
                | PythonValue::SortedMap(_) => {
                    panic!("Maps cannot be used as map keys.")
                }
                PythonValue::CdtSpecial(ref s) => s.hash(state),
            }
        }
    }

    impl PythonValue {
        /// Returns a string representation of the value.
        pub fn as_string(&self) -> String {
            match *self {
                PythonValue::Nil => "<null>".to_string(),
                PythonValue::Int(ref val) => val.to_string(),
                PythonValue::Bool(ref val) => val.to_string(),
                PythonValue::Float(ref val) => val.to_string(),
                PythonValue::String(ref val) => val.to_string(),
                PythonValue::GeoJSON(ref val) => format!("GeoJSON('{}')", val),
                PythonValue::Blob(ref val) => format!("{:?}", val),
                PythonValue::HLL(ref val) => format!("HLL('{:?}')", val),
                PythonValue::Vector(ref val) => val.to_string(),
                PythonValue::List(ref val) => format!("{:?}", val),
                PythonValue::HashMap(ref val) => format!("{:?}", val),
                PythonValue::SortedMap(ref val) => format!("{:?}", val),
                PythonValue::OrderedMap(ref val) => format!("{:?}", val),
                PythonValue::CdtSpecial(s) => match s {
                    SpecialValue::Null => "SpecialValue.NULL".to_string(),
                    SpecialValue::Infinity => "SpecialValue.INFINITY".to_string(),
                    SpecialValue::Wildcard => "SpecialValue.WILDCARD".to_string(),
                },
            }
        }
    }

    impl fmt::Display for PythonValue {
        fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
            write!(f, "{}", self.as_string())
        }
    }

    impl<'py> IntoPyObject<'py> for PythonValue {
        type Target = PyAny; // the Python type
        type Output = Bound<'py, Self::Target>; // in most cases this will be `Bound`
        type Error = std::convert::Infallible;

        fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
            match self {
                PythonValue::Nil => Ok(py.None().into_bound(py)),
                PythonValue::Bool(b) => Ok(PyBool::new(py, b).into_bound_py_any(py).unwrap()),
                PythonValue::Int(i) => Ok(i.into_pyobject(py).map(|v| v.into_any()).unwrap()),
                PythonValue::Float(f) => Ok(f.into_pyobject(py).map(|v| v.into_any()).unwrap()),
                PythonValue::String(s) => Ok(s.into_pyobject(py).map(|v| v.into_any()).unwrap()),
                PythonValue::Blob(b) => Ok(PyBytes::new(py, &b).into_any()),
                PythonValue::List(l) => {
                    let py_list = PyList::empty(py);
                    for item in l {
                        let py_item = item.into_pyobject(py).unwrap();
                        py_list.append(py_item).unwrap();
                    }
                    Ok(py_list.into_any())
                }
                PythonValue::SortedMap(pairs) => {
                    let py_dict = PyDict::new(py);
                    for (k, v) in pairs {
                        let py_key = k.into_pyobject(py).unwrap();
                        let py_val = v.into_pyobject(py).unwrap();
                        py_dict.set_item(py_key, py_val).unwrap();
                    }
                    // Construct the declared type rather than a bare dict: the
                    // class is a dict subclass, so this stays a drop-in for one
                    // while telling the caller the map is key-ordered.
                    let cls = SortedMap::type_object(py);
                    Ok(cls.call1((py_dict,)).unwrap().into_any())
                }
                PythonValue::HashMap(h) => {
                    let py_dict = PyDict::new(py);
                    for (k, v) in h {
                        let py_key = k.into_pyobject(py).unwrap();
                        let py_val = v.into_pyobject(py).unwrap();
                        py_dict.set_item(py_key, py_val).unwrap();
                    }
                    Ok(py_dict.into_any())
                }
                PythonValue::OrderedMap(pairs) => {
                    let py_dict = PyDict::new(py);
                    for (k, v) in pairs {
                        let py_key = k.into_pyobject(py).unwrap();
                        let py_val = v.into_pyobject(py).unwrap();
                        py_dict.set_item(py_key, py_val).unwrap();
                    }
                    Ok(py_dict.into_any())
                }
                PythonValue::GeoJSON(s) => {
                    let geo = GeoJSON { v: s };
                    Ok(geo.into_pyobject(py).map(|v| v.into_any()).unwrap())
                }
                PythonValue::HLL(b) => Ok(HLL::new(b).into_pyobject(py).map(|v| v.into_any()).unwrap()),
                PythonValue::Vector(v) => Ok(Vector { v }.into_pyobject(py).map(|v| v.into_any()).unwrap()),
                PythonValue::CdtSpecial(s) => {
                    Ok(s.into_pyobject(py).map(|v| v.into_any()).unwrap())
                }
            }
        }
    }

    impl<'a, 'py> FromPyObject<'a, 'py> for PythonValue {
        type Error = PyErr;

        fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
            // Handle None first - check if the object is None
            if obj.is_none() {
                return Ok(PythonValue::Nil);
            }

            // Ahead of the sequence extraction below, which would otherwise read a
            // Vector as a list of numbers.
            if let Ok(vector) = obj.cast::<Vector>() {
                return Ok(PythonValue::Vector(vector.get().v.clone()));
            }

            if let Ok(sv) = <SpecialValue as FromPyObject>::extract(obj) {
                return Ok(PythonValue::CdtSpecial(sv));
            }

            let b: PyResult<bool> = obj.extract();
            if let Ok(b) = b {
                return Ok(PythonValue::Bool(b));
            }

            // Try to extract as integer - handle both i64 and large u64 values
            // First try i64 (most common case)
            let i: PyResult<i64> = obj.extract();
            if let Ok(i) = i {
                return Ok(PythonValue::Int(i));
            }

            // For u64 values, convert to i64 (UInt has been removed from Rust core)
            // Values > i64::MAX will overflow, but this matches Rust core behavior
            let ui: PyResult<u64> = obj.extract();
            if let Ok(ui) = ui {
                // Convert u64 to i64 (may overflow for values > i64::MAX)
                return Ok(PythonValue::Int(ui as i64));
            }

            let f1: PyResult<f64> = obj.extract();
            if let Ok(f1) = f1 {
                return Ok(PythonValue::Float(ordered_float::OrderedFloat(f1)));
            }

            let s: PyResult<String> = obj.extract();
            if let Ok(s) = s {
                return Ok(PythonValue::String(s));
            }

            // Try to extract as bytearray
            if let Ok(ba) = obj.cast::<PyByteArray>() {
                return Ok(PythonValue::Blob(ba.to_vec()));
            }

            // Try to extract as bytes
            if let Ok(bytes) = obj.cast::<PyBytes>() {
                return Ok(PythonValue::Blob(bytes.as_bytes().to_vec()));
            }

            let b: Result<Blob, _> = obj.extract();
            if let Ok(b) = b {
                return Ok(PythonValue::Blob(b.v));
            }

            let l: PyResult<Vec<PythonValue>> = obj.extract();
            if let Ok(l) = l {
                return Ok(PythonValue::List(l));
            }

            let l: Result<List, _> = obj.extract();
            if let Ok(l) = l {
                return Ok(PythonValue::List(l.v));
            }

            // Before the plain-dict arm: a SortedMap must not degrade to an
            // unordered map just because it also presents as a mapping.
            if obj.is_instance_of::<SortedMap>() {
                // A dict subclass, so the entries come from the base type. Read
                // as pairs rather than through a HashMap so the order survives.
                let as_dict: &Bound<'_, PyDict> = obj.as_any().cast()?;
                let mut pairs = Vec::with_capacity(as_dict.len());
                for (k, v) in as_dict.iter() {
                    pairs.push((k.extract()?, v.extract()?));
                }
                return Ok(PythonValue::SortedMap(pairs));
            }

            let hm: PyResult<HashMap<PythonValue, PythonValue>> = obj.extract();
            if let Ok(hm) = hm {
                return Ok(PythonValue::HashMap(hm));
            }

            let geo: Result<GeoJSON, _> = obj.extract();
            if let Ok(geo) = geo {
                return Ok(PythonValue::GeoJSON(geo.v));
            }

            let hll: Result<HLL, _> = obj.extract();
            if let Ok(hll) = hll {
                return Ok(PythonValue::HLL(hll.v));
            }

            Err(PyTypeError::new_err("invalid value type"))
        }
    }

    impl From<HashMap<String, aerospike_core::Value>> for PythonValue {
        fn from(h: HashMap<String, aerospike_core::Value>) -> Self {
            let mut hash = HashMap::<PythonValue, PythonValue>::with_capacity(h.len());
            h.iter().for_each(|(k, v)| {
                hash.insert(PythonValue::String(k.into()), v.clone().into());
            });
            PythonValue::HashMap(hash)
        }
    }

    impl From<PythonValue> for aerospike_core::Value {
        fn from(other: PythonValue) -> Self {
            match other {
                PythonValue::Nil => aerospike_core::Value::Nil,
                PythonValue::Bool(b) => aerospike_core::Value::Bool(b),
                PythonValue::Int(i) => aerospike_core::Value::Int(i),
                PythonValue::Float(f) => aerospike_core::Value::Float(f64::from(f).into()),
                PythonValue::String(s) => aerospike_core::Value::String(s),
                PythonValue::Blob(b) => aerospike_core::Value::Blob(b),
                PythonValue::List(l) => {
                    let mut nl = Vec::<aerospike_core::Value>::with_capacity(l.len());
                    l.iter().for_each(|v| nl.push(v.clone().into()));
                    aerospike_core::Value::List(nl)
                }
                PythonValue::HashMap(h) => {
                    let mut arr = HashMap::with_capacity(h.len());
                    h.iter().for_each(|(k, v)| {
                        arr.insert(k.clone().into(), v.clone().into());
                    });
                    aerospike_core::Value::HashMap(arr)
                }
                PythonValue::SortedMap(pairs) => {
                    // BTreeMap is what makes this key-ordered: core packs it with
                    // the K_ORDERED flag and the entries already in key order,
                    // which the server requires for that flag.
                    let mut btree = std::collections::BTreeMap::new();
                    for (key, val) in pairs {
                        btree.insert(
                            aerospike_core::Value::from(key),
                            aerospike_core::Value::from(val),
                        );
                    }
                    aerospike_core::Value::SortedMap(btree)
                }
                PythonValue::OrderedMap(pairs) => {
                    // Insertion-ordered on the Python side maps to the core
                    // insertion-ordered map (IndexMap), preserving pair order
                    // on the wire.
                    let mut map = aerospike_core::IndexMap::with_capacity(pairs.len());
                    for (k, v) in pairs {
                        map.insert(k.into(), v.into());
                    }
                    aerospike_core::Value::OrderedMap(map)
                }
                PythonValue::GeoJSON(gj) => aerospike_core::Value::GeoJson(gj),
                PythonValue::HLL(b) => aerospike_core::Value::Hll(b),
                PythonValue::Vector(v) => aerospike_core::Value::Vector(v),
                PythonValue::CdtSpecial(s) => match s {
                    SpecialValue::Null => aerospike_core::Value::Nil,
                    SpecialValue::Infinity => aerospike_core::Value::Infinity,
                    SpecialValue::Wildcard => aerospike_core::Value::Wildcard,
                },
            }
        }
    }

    impl From<aerospike_core::Value> for PythonValue {
        fn from(other: aerospike_core::Value) -> Self {
            match other {
                aerospike_core::Value::Nil => PythonValue::Nil,
                aerospike_core::Value::Bool(b) => PythonValue::Bool(b),
                aerospike_core::Value::Int(i) => PythonValue::Int(i),
                aerospike_core::Value::Float(fv) => {
                    PythonValue::Float(ordered_float::OrderedFloat(fv.into()))
                }
                aerospike_core::Value::String(s) => PythonValue::String(s),
                aerospike_core::Value::Blob(b) => PythonValue::Blob(b),
                aerospike_core::Value::List(l) => {
                    let mut nl = Vec::<PythonValue>::with_capacity(l.len());
                    l.iter().for_each(|v| nl.push(v.clone().into()));
                    PythonValue::List(nl)
                }
                aerospike_core::Value::MultiResult(mv) => {
                    // MultiResult is returned when server executes multiple operations for the same bin
                    // Convert to a list of PythonValues without flattening
                    let mut nl = Vec::<PythonValue>::with_capacity(mv.len());
                    mv.iter().for_each(|v| nl.push(v.clone().into()));
                    PythonValue::List(nl)
                }
                aerospike_core::Value::HashMap(h) => {
                    let mut arr = HashMap::with_capacity(h.len());
                    h.iter().for_each(|(k, v)| {
                        arr.insert(k.clone().into(), v.clone().into());
                    });
                    PythonValue::HashMap(arr)
                }
                aerospike_core::Value::OrderedMap(om) => {
                    let pairs: Vec<(PythonValue, PythonValue)> = om
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect();
                    PythonValue::OrderedMap(pairs)
                }
                aerospike_core::Value::SortedMap(sm) => {
                    // K-ordered server return: hand back the declared type, so a
                    // map written as a SortedMap reads back as one. BTreeMap
                    // iteration is already in the server's canonical key order.
                    let pairs: Vec<(PythonValue, PythonValue)> = sm
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect();
                    PythonValue::SortedMap(pairs)
                }
                aerospike_core::Value::Unknown(_code, bytes) => {
                    // Particle types this client does not interpret (legacy
                    // language-specific serializations, retired types). Surface
                    // the raw payload so the data is still accessible.
                    PythonValue::Blob(bytes)
                }
                aerospike_core::Value::GeoJson(gj) => PythonValue::GeoJSON(gj),
                aerospike_core::Value::Hll(b) => PythonValue::HLL(b),
                aerospike_core::Value::Vector(v) => PythonValue::Vector(v),
                aerospike_core::Value::Infinity => PythonValue::CdtSpecial(SpecialValue::Infinity),
                aerospike_core::Value::Wildcard => PythonValue::CdtSpecial(SpecialValue::Wildcard),
                aerospike_core::Value::KeyValueList(kvl) => {
                    let pairs: Vec<(PythonValue, PythonValue)> = kvl
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect();
                    PythonValue::OrderedMap(pairs)
                }
                other => unreachable!("unmapped core Value: {other:?}"),
            }
        }
    }

    impl PyStubType for PythonValue {
        fn type_output() -> TypeInfo {
            TypeInfo::any()
        }
    }

    // impl From<aerospike_core::Bin> for Bin {
    //     fn from(other: aerospike_core::Bin) -> Self {
    //         Bin { _as: other }
    //     }
    // }

    impl From<aerospike_core::Key> for Key {
        fn from(other: aerospike_core::Key) -> Self {
            Key { _as: other }
        }
    }

    impl From<aerospike_core::Record> for Record {
        fn from(other: aerospike_core::Record) -> Self {
            Record { _as: other, cached_bins: None, cached_results: None }
        }
    }
