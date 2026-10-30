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
use std::fmt;
use std::hash::{Hash, Hasher};

use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::exceptions::PyTypeError;
use pyo3::types::{PyBool, PyByteArray, PyBytes, PyDict, PyList, PySuper, PyTuple};
use pyo3::PyTypeInfo;
use pyo3::{prelude::*, Borrowed, IntoPyObjectExt};

use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};
use pyo3_stub_gen::{PyStubType, TypeInfo};



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
        /// The ``bins`` dict, built on first access. Building it moves the
        /// values out of ``_as.bins``, so once set it holds the only copy.
        pub(crate) cached_bins: Option<Py<PyDict>>,
        /// The ``results`` list, built on first access. Building it moves the
        /// values out of ``_as.results``, so once set it holds the only copy.
        pub(crate) cached_results: Option<Py<PyList>>,
    }

    impl Clone for Record {
        fn clone(&self) -> Self {
            // A built cache holds the only copy of its values, so the clone
            // takes a shallow copy of it rather than starting empty.
            Python::attach(|py| Record {
                _as: self._as.clone(),
                cached_bins: self.cached_bins.as_ref().map(|d| d.bind(py).copy().unwrap().unbind()),
                cached_results: self
                    .cached_results
                    .as_ref()
                    .map(|l| PyList::new(py, l.bind(py).iter()).unwrap().unbind()),
            })
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Record {
        pub fn bin(&self, py: Python<'_>, name: &str) -> PyResult<Option<Py<PyAny>>> {
            if let Some(ref bins) = self.cached_bins {
                return Ok(bins.bind(py).get_item(name)?.map(Bound::unbind));
            }
            let Some(v) = self._as.bins.get(name) else {
                return Ok(None);
            };
            let pv: PythonValue = v.to_owned().into();
            Ok(Some(pv.into_pyobject(py)?.unbind()))
        }

        #[getter]
        pub fn get_bins(&mut self, py: Python<'_>) -> Py<PyAny> {
            if let Some(ref cached) = self.cached_bins {
                return cached.clone_ref(py).into_any();
            }
            let dict = PyDict::new(py);
            for (k, v) in std::mem::take(&mut self._as.bins) {
                let pv: PythonValue = v.into();
                dict.set_item(k, pv.into_pyobject(py).unwrap()).unwrap();
            }
            let dict = dict.unbind();
            self.cached_bins = Some(dict.clone_ref(py));
            dict.into_any()
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
                return Some(cached.clone_ref(py).into_any());
            }
            let results = self._as.results.take()?;
            let list = PyList::empty(py);
            for v in results {
                let pv: PythonValue = v.into();
                list.append(pv.into_pyobject(py).unwrap()).unwrap();
            }
            let list = list.unbind();
            self.cached_results = Some(list.clone_ref(py));
            Some(list.into_any())
        }

        /// Return the positional result for the *i*-th op in the request,
        /// or ``None`` if *i* is out of range.
        ///
        /// Equivalent to ``record.results[i]`` but without bounds-error noise:
        /// out-of-range returns Python ``None`` (matching the in-range
        /// nil-result encoding). Use this when the request size is dynamic
        /// and the caller wants a uniform optional shape.
        pub fn operation_result(&self, py: Python<'_>, i: usize) -> PyResult<Option<Py<PyAny>>> {
            if let Some(ref results) = self.cached_results {
                let results = results.bind(py);
                if i >= results.len() {
                    return Ok(None);
                }
                return Ok(Some(results.get_item(i)?.unbind()));
            }
            let Some(v) = self._as.operation_result(i) else {
                return Ok(None);
            };
            let pv: PythonValue = v.to_owned().into();
            Ok(Some(pv.into_pyobject(py)?.unbind()))
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

        fn __str__(&mut self, py: Python<'_>) -> PyResult<String> {
            let ttl = match self._as.time_to_live() {
                None => "None".to_string(),
                Some(duration) => duration.as_secs().to_string(),
            };
            let bins = self.get_bins(py).bind(py).repr()?.to_string();
            Ok(format!(
                "generation: {}, ttl: {}, key: {:?}, bins: {}",
                self._as.generation, ttl, self._as.key, bins
            ))
        }

        fn __repr__(&mut self, py: Python<'_>) -> PyResult<String> {
            Ok(format!("Record({})", self.__str__(py)?))
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
