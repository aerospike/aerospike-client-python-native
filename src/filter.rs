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

use std::fmt;
use std::pin::Pin;
use std::sync::Arc;

use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::exceptions::PyStopAsyncIteration;
use pyo3::types::{PyBool, PyList};
use pyo3::{prelude::*, IntoPyObjectExt};

use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use tokio::sync::Mutex;

use aerospike_core::query::RecordStream;

use crate::{OrderByType, CollectionIndexType, Order};
use crate::cdt::CTX;
use crate::errors::RustClientError;
use crate::expressions::FilterExpression;
use crate::operations::bins_flag;
use crate::record::{Key, PythonValue, Record};

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  PartitionStatus
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(name = "PartitionStatus", module = "_native")]
    #[derive(Debug)]
    pub struct PartitionStatus {
        pub(crate) _as: aerospike_core::query::PartitionStatus,
    }

    // Note: PartitionStatus can be constructed from Python using PartitionStatus(id)
    // Users typically get PartitionStatus instances from query/scan operations,
    // but can also create new instances manually when needed.

    /// Builds a core status carrying the given cursor. The delivery
    /// accounting behind core's private fields starts zeroed, which is what a
    /// rebuilt cursor needs: a (re)query round resets it before use.
    fn core_partition_status(
        id: u16,
        retry: bool,
        bval: Option<u64>,
        digest: Option<[u8; 20]>,
        node: Option<std::sync::Arc<aerospike_core::Node>>,
        sequence: Option<u64>,
    ) -> aerospike_core::query::PartitionStatus {
        let mut status = aerospike_core::query::PartitionStatus::new(id as usize);
        status.retry = retry;
        status.bval = bval;
        status.digest = digest;
        status.node = node;
        status.sequence = sequence;
        status
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl PartitionStatus {
        /// Create a new PartitionStatus with the specified partition ID.
        ///
        /// The `retry` field defaults to `true`, and other fields can be set via setters.
        #[new]
        pub fn new(id: u16) -> Self {
            PartitionStatus {
                _as: core_partition_status(id, true, None, None, None, None),
            }
        }

        #[getter]
        pub fn get_bval(&self) -> Option<u64> {
            self._as.bval
        }

        #[setter]
        pub fn set_bval(&mut self, bval: Option<u64>) {
            self._as.bval = bval;
        }

        #[getter]
        pub fn get_id(&self) -> u16 {
            self._as.id
        }

        #[getter]
        pub fn get_retry(&self) -> bool {
            self._as.retry
        }

        #[setter]
        pub fn set_retry(&mut self, retry: bool) {
            self._as.retry = retry;
        }

        #[getter]
        pub fn get_digest(&self) -> Option<String> {
            self._as.digest.map(hex::encode)
        }

        #[setter]
        pub fn set_digest(&mut self, digest: Option<String>) -> PyResult<()> {
            match digest {
                None => {
                    self._as.digest = None;
                }
                Some(hex_str) => {
                    let bytes = hex::decode(&hex_str)
                        .map_err(|e| PyValueError::new_err(format!("Invalid hex digest: {}", e)))?;
                    if bytes.len() != 20 {
                        return Err(PyValueError::new_err(format!(
                            "Digest must be exactly 20 bytes (40 hex chars), got {} bytes",
                            bytes.len()
                        )));
                    }
                    let mut digest_array = [0u8; 20];
                    digest_array.copy_from_slice(&bytes);
                    self._as.digest = Some(digest_array);
                }
            }
            Ok(())
        }


        /// Dictionary-style access for convenience (in addition to getters/setters).
        /// Supported keys: 'id', 'bval', 'retry', 'digest'
        /// Example: ps['id'], ps['bval'] = 123
        pub fn __getitem__(&self, key: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
            let py = key.py();
            let key_str = key.extract::<String>()?;
            match key_str.as_str() {
                "id" => Ok(self.get_id().into_pyobject(py).unwrap().into_any().into()),
                "bval" => match self.get_bval() {
                    Some(v) => Ok(v.into_pyobject(py).unwrap().into_any().into()),
                    None => Ok(py.None()),
                },
                "retry" => Ok(PyBool::new(py, self.get_retry()).into_bound_py_any(py).unwrap().into()),
                "digest" => match self.get_digest() {
                    Some(v) => Ok(v.into_pyobject(py).unwrap().into_any().into()),
                    None => Ok(py.None()),
                },
                _ => Err(PyKeyError::new_err(format!("Unknown key: '{}'. Valid keys: 'id', 'bval', 'retry', 'digest'", key_str))),
            }
        }

        /// Dictionary-style assignment for convenience (in addition to getters/setters).
        /// Supported keys: 'bval', 'retry', 'digest'
        /// Note: 'id' is read-only and cannot be set.
        pub fn __setitem__(&mut self, key: &Bound<'_, PyAny>, value: &Bound<'_, PyAny>) -> PyResult<()> {
            let key_str = key.extract::<String>()?;
            match key_str.as_str() {
                "id" => Err(PyValueError::new_err("'id' is read-only and cannot be set")),
                "bval" => {
                    let bval: Option<u64> = value.extract()?;
                    self.set_bval(bval);
                    Ok(())
                }
                "retry" => {
                    let retry: bool = value.extract()?;
                    self.set_retry(retry);
                    Ok(())
                }
                "digest" => {
                    let digest: Option<String> = value.extract()?;
                    self.set_digest(digest)?;
                    Ok(())
                }
                _ => Err(PyKeyError::new_err(format!("Unknown key: '{}'. Valid keys: 'bval', 'retry', 'digest'", key_str))),
            }
        }
    }

    //  PartitionFilter
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, 
        name = "PartitionFilter",
        module = "_native",
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Debug, Clone)]
    pub struct PartitionFilter {
        pub(crate) _as: aerospike_core::query::PartitionFilter,
    }

    impl Default for PartitionFilter {
        fn default() -> Self {
            PartitionFilter {
                _as: aerospike_core::query::PartitionFilter::all(),
            }
        }
    }

    /// Trait implemented by most policy types; policies that implement this trait typically encompass
    /// an instance of `PartitionFilter`.
    #[gen_stub_pymethods]
    #[pymethods]
    impl PartitionFilter {
        #[new]
        pub fn new() -> Self {
            Self::default()
        }

        pub fn done(&self) -> bool {
            self._as.done()
        }

        #[staticmethod]
        pub fn all() -> Self {
            Self {
                _as: aerospike_core::query::PartitionFilter::all(),
            }
        }

        #[staticmethod]
        pub fn by_id(id: usize) -> Self {
            Self {
                _as: aerospike_core::query::PartitionFilter::by_id(id),
            }
        }

        #[staticmethod]
        pub fn by_key(key: &Key) -> Self {
            Self {
                _as: aerospike_core::query::PartitionFilter::by_key(&key._as),
            }
        }

        #[staticmethod]
        pub fn by_range(begin: usize, count: usize) -> Self {
            Self {
                _as: aerospike_core::query::PartitionFilter::by_range(begin, count),
            }
        }

        #[getter]
        pub fn get_begin(&self) -> usize {
            self._as.begin
        }

        #[setter]
        pub fn set_begin(&mut self, begin: usize) {
            self._as.begin = begin;
        }

        #[getter]
        pub fn get_count(&self) -> usize {
            self._as.count
        }

        #[setter]
        pub fn set_count(&mut self, count: usize) {
            self._as.count = count;
        }

        #[getter]
        pub fn get_digest(&self) -> Option<String> {
            self._as.digest.map(hex::encode)
        }

        #[setter]
        pub fn set_digest(&mut self, digest: Option<String>) -> PyResult<()> {
            match digest {
                None => {
                    self._as.digest = None;
                }
                Some(hex_str) => {
                    let bytes = hex::decode(&hex_str)
                        .map_err(|e| PyValueError::new_err(format!("Invalid hex digest: {}", e)))?;
                    if bytes.len() != 20 {
                        return Err(PyValueError::new_err(format!(
                            "Digest must be exactly 20 bytes (40 hex chars), got {} bytes",
                            bytes.len()
                        )));
                    }
                    let mut digest_array = [0u8; 20];
                    digest_array.copy_from_slice(&bytes);
                    self._as.digest = Some(digest_array);
                }
            }
            Ok(())
        }

        #[getter]
        pub fn get_partitions(&self, py: Python) -> PyResult<Py<PyAny>> {
            match &self._as.partitions {
                None => Ok(py.None()),
                Some(partitions) => {
                    let mut py_partitions = Vec::new();
                    for arc_mutex_status in partitions.iter() {
                        // parking_lot Mutex: a synchronous lock, so no Tokio runtime
                        // handle is needed — works from a Python asyncio context.
                        let status = arc_mutex_status.lock();
                        let py_status = PartitionStatus {
                            _as: core_partition_status(
                                status.id,
                                status.retry,
                                status.bval,
                                status.digest,
                                status.node.clone(),
                                status.sequence,
                            ),
                        };
                        py_partitions.push(Py::new(py, py_status)?);
                    }
                    let list = PyList::empty(py);
                    for item in py_partitions {
                        list.append(item)?;
                    }
                    Ok(list.into())
                }
            }
        }

        #[setter]
        pub fn set_partitions(&mut self, partitions: Option<Bound<'_, PyList>>) -> PyResult<()> {
            match partitions {
                None => {
                    self._as.partitions = None;
                }
                Some(py_partitions) => {
                    let mut rust_partitions = Vec::new();
                    for item in py_partitions.iter() {
                        let status: PyRef<PartitionStatus> = item.extract()?;
                        // node/sequence are deliberately dropped: they name
                        // cluster topology from the cursor's previous life and
                        // must be re-resolved by the query that resumes it.
                        rust_partitions.push(parking_lot::Mutex::new(core_partition_status(
                            status._as.id,
                            status._as.retry,
                            status._as.bval,
                            status._as.digest,
                            None,
                            None,
                        )));
                    }
                    self._as.partitions = Some(Arc::new(rust_partitions));
                }
            }
            Ok(())
        }
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Statement
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Query statement parameters.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, 
        name = "Statement",
        module = "_native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone)]
    pub struct Statement {
        pub(crate) _as: aerospike_core::Statement,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Statement {
        #[new]
        #[pyo3(signature = (namespace, set_name = None, bins = None))]
        pub fn __construct(
            namespace: &str,
            set_name: Option<&str>,
            bins: Option<Vec<String>>,
        ) -> Self {
            let set_name_str = set_name.unwrap_or("");
            Statement {
                _as: aerospike_core::Statement::new(namespace, set_name_str, bins_flag(bins)),
            }
        }

        #[getter]
        pub fn get_filters(&self) -> Option<Vec<Filter>> {
            self._as.filter.as_ref().map(|f| vec![Filter { _as: f.clone() }])
        }

        /// The server accepts one secondary-index filter per query.
        #[setter]
        pub fn set_filters(&mut self, filters: Option<Vec<Filter>>) -> PyResult<()> {
            let mut filters = filters.unwrap_or_default();
            if filters.len() > 1 {
                return Err(PyValueError::new_err(
                    "a query accepts at most one secondary-index filter",
                ));
            }
            self._as.filter = filters.pop().map(|f| f._as);
            Ok(())
        }

        #[getter]
        pub fn get_set_name(&self) -> Option<String> {
            if self._as.set_name.is_empty() {
                None
            } else {
                Some(self._as.set_name.clone())
            }
        }

        #[setter]
        pub fn set_set_name(&mut self, set_name: Option<String>) {
            self._as.set_name = set_name.unwrap_or_default();
        }

        /// Set Lua aggregation function parameters for query aggregation.
        ///
        /// Args:
        ///     package_name: Name of the Lua package/module containing the aggregation function.
        ///     function_name: Name of the Lua aggregation function.
        ///     function_args: Optional list of arguments to pass to the function.
        #[pyo3(signature = (package_name, function_name, function_args = None))]
        pub fn set_aggregate_function(
            &mut self,
            package_name: &str,
            function_name: &str,
            function_args: Option<Vec<PythonValue>>,
        ) {
            let args: Vec<aerospike_core::Value> = function_args
                .map(|args| args.into_iter().map(|v| v.into()).collect())
                .unwrap_or_default();
            self._as.set_aggregate_function(package_name, function_name, &args);
        }

        /// Attach an ops projection. The server returns the result of these
        /// operations for each matching record instead of the bin set
        /// configured via ``bins``. Mutually exclusive with ``bins`` (the
        /// server uses ``operations`` if both are set).
        ///
        /// Foreground queries accept only read ops. Server versions before
        /// 8.1.2 only accept the basic ``Read`` op here; 8.1.2+ also accepts
        /// CDT, expression, bit, and HLL reads.
        pub fn set_operations(&mut self, py: Python<'_>, ops: Vec<Py<PyAny>>) -> PyResult<()> {
            let py_ops_with_ctx = crate::operations::extract_py_ops_with_ctx(py, &ops)?;
            let (core_ops, _has_write) =
                crate::operations::convert_ops_with_ctx_to_core(&py_ops_with_ctx, false)?;
            self._as.set_operations(core_ops);
            Ok(())
        }

        /// Sets the order key of a Top-K query, replacing any keys set before.
        ///
        /// A Top-K query returns the ``k`` best records by this key (see
        /// :attr:`top_k`), best first. Every node ranks its own records and the
        /// client merges them; a record that two nodes both returned keeps its
        /// later-updated copy. Ranking runs after the index filter and
        /// ``QueryPolicy.filter_expression``, so it changes which matches come
        /// back, not which records match.
        ///
        /// ``bin_name`` names a bin of the *returned* record: a stored bin named in
        /// ``bins``, or one produced by exactly one :meth:`set_operations` read,
        /// such as an ``ExpOperation.read`` of :meth:`FilterExpression.vector_distance`.
        /// A record whose key is missing or of another type ranks last in either
        /// direction; ties break by digest.
        ///
        /// Checked when the query runs, which raises :class:`ValueError` for a bin
        /// name that is empty, longer than 15 UTF-8 bytes or holds a NUL; a key the
        /// projection does not return; or an order key without :attr:`top_k`.
        /// Top-K requires every node to run Aerospike Server 8.2.1 or later, and
        /// cannot be combined with an aggregation, a background query,
        /// ``QueryPolicy.max_records`` or a resumed partition filter.
        pub fn set_order_by(&mut self, bin_name: &str, data_type: OrderByType, order: Order) {
            self._as.set_order_by(bin_name, data_type.into(), order.into());
        }

        /// Adds a second order key, compared when the first key ties. It must name
        /// a different bin; a Top-K query takes at most two keys. See
        /// :meth:`set_order_by`.
        pub fn then_order_by(&mut self, bin_name: &str, data_type: OrderByType, order: Order) {
            self._as.then_order_by(bin_name, data_type.into(), order.into());
        }

        /// The Top-K order keys, in comparison order; empty for an unranked query.
        #[getter]
        pub fn get_order_by(&self) -> Vec<OrderBy> {
            self._as.order_by.iter().map(|key| OrderBy { _as: key.clone() }).collect()
        }

        /// The Top-K limit ``k``, in ``[1, 1000]``, or ``None`` for an unranked
        /// query. Requires an order key (see :meth:`set_order_by`); the two may be
        /// set in either order.
        #[getter]
        pub fn get_top_k(&self) -> Option<u32> {
            self._as.top_k
        }

        #[setter]
        pub fn set_top_k(&mut self, k: Option<u32>) {
            self._as.top_k = k;
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  OrderBy
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// One Top-K order key of a :class:`Statement`, set with
    /// :meth:`Statement.set_order_by` or :meth:`Statement.then_order_by`.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(frozen, skip_from_py_object, name = "OrderBy", module = "_native")]
    #[derive(Clone)]
    pub struct OrderBy {
        _as: aerospike_core::OrderBy,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl OrderBy {
        /// Name of the returned bin the key reads.
        #[getter]
        pub fn bin_name(&self) -> String {
            self._as.bin_name.clone()
        }

        /// Declared type of the key.
        #[getter]
        pub fn data_type(&self) -> OrderByType {
            self._as.order_type.into()
        }

        /// Sort direction.
        #[getter]
        pub fn order(&self) -> Order {
            self._as.direction.into()
        }

        fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
            other.cast::<OrderBy>().is_ok_and(|o| o.get()._as == self._as)
        }

        fn __hash__(&self) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let key = &self._as;
            let mut hasher = DefaultHasher::new();
            (&key.bin_name, key.order_type, key.direction).hash(&mut hasher);
            hasher.finish()
        }

        fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
            Ok(format!(
                "OrderBy({}, {}, {})",
                self._as.bin_name.as_str().into_pyobject(py)?.repr()?,
                Bound::new(py, self.data_type())?.as_any().repr()?,
                Bound::new(py, self.order())?.as_any().repr()?,
            ))
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Filter
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Query filter definition. Currently, only one filter is allowed in a Statement, and must target a
    /// bin that has a secondary index (or use `*_by_index` with the index name).
    ///
    /// Build filters from the class static methods, for example `equal`, `range`, `contains`,
    /// `contains_range`, `within_region`, `within_radius`, `regions_containing_point`, and the
    /// corresponding `equal_by_index`, `range_by_index`, `contains_by_index`, `contains_range_by_index`,
    /// `within_region_by_index`, `within_radius_by_index`, and `regions_containing_point_by_index`.
    ///
    /// Use instance methods `context` and `expression` to attach a CDT path or expression-based index
    /// to a filter (for example `Filter.equal("bin", 1).context([CTX.list_index(0)])`).
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, 
        name = "Filter",
        module = "_native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    #[derive(Clone, Debug)]
    pub struct Filter {
        pub(crate) _as: aerospike_core::query::Filter,
    }

    impl fmt::Display for Filter {
        fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            write!(f, "Filter({:?})", self._as)
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Filter {
        fn __str__(&self) -> PyResult<String> {
            Ok(format!("{}", self))
        }

        fn __repr__(&self) -> PyResult<String> {
            Ok(format!("Filter({:?})", self._as))
        }

        /// Attach a CDT context path for a secondary index on a nested list or map element.
        pub fn context(&self, ctx: Vec<CTX>) -> Self {
            let core_ctx = crate::cdt::ctx_to_vec(&ctx);
            Filter {
                _as: self._as.clone().context(core_ctx),
            }
        }

        /// Attach the expression used when the secondary index was created with
        /// `create_index_using_expression`.
        pub fn expression(&self, exp: &FilterExpression) -> Self {
            Filter {
                _as: self._as.clone().expression(exp._as.clone()),
            }
        }

        #[staticmethod]
        pub fn equal(bin_name: &str, value: PythonValue) -> Self {
            Filter {
                _as: aerospike_core::query::Filter::equal(
                    bin_name,
                    aerospike_core::Value::from(value),
                ),
            }
        }

        #[staticmethod]
        pub fn equal_by_index(index_name: &str, value: PythonValue) -> Self {
            Filter {
                _as: aerospike_core::query::Filter::equal_by_index(
                    index_name,
                    aerospike_core::Value::from(value),
                ),
            }
        }

        #[staticmethod]
        pub fn range(bin_name: &str, begin: PythonValue, end: PythonValue) -> Self {
            Filter {
                _as: aerospike_core::query::Filter::range(
                    bin_name,
                    aerospike_core::Value::from(begin),
                    aerospike_core::Value::from(end),
                ),
            }
        }

        #[staticmethod]
        pub fn range_by_index(index_name: &str, begin: PythonValue, end: PythonValue) -> Self {
            Filter {
                _as: aerospike_core::query::Filter::range_by_index(
                    index_name,
                    aerospike_core::Value::from(begin),
                    aerospike_core::Value::from(end),
                ),
            }
        }

        #[staticmethod]
        pub fn contains(
            bin_name: &str,
            value: PythonValue,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            Filter {
                _as: aerospike_core::query::Filter::contains(
                    bin_name,
                    aerospike_core::Value::from(value),
                    aerospike_core::query::CollectionIndexType::from(cit),
                ),
            }
        }

        #[staticmethod]
        pub fn contains_by_index(
            index_name: &str,
            value: PythonValue,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            Filter {
                _as: aerospike_core::query::Filter::contains_by_index(
                    index_name,
                    aerospike_core::Value::from(value),
                    aerospike_core::query::CollectionIndexType::from(cit),
                ),
            }
        }

        #[staticmethod]
        pub fn contains_range(
            bin_name: &str,
            begin: PythonValue,
            end: PythonValue,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            Filter {
                _as: aerospike_core::query::Filter::contains_range(
                    bin_name,
                    aerospike_core::Value::from(begin),
                    aerospike_core::Value::from(end),
                    aerospike_core::query::CollectionIndexType::from(cit),
                ),
            }
        }

        #[staticmethod]
        pub fn contains_range_by_index(
            index_name: &str,
            begin: PythonValue,
            end: PythonValue,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            Filter {
                _as: aerospike_core::query::Filter::contains_range_by_index(
                    index_name,
                    aerospike_core::Value::from(begin),
                    aerospike_core::Value::from(end),
                    aerospike_core::query::CollectionIndexType::from(cit),
                ),
            }
        }

        #[staticmethod]
        pub fn within_region(
            bin_name: &str,
            region: &str,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            if matches!(cit, CollectionIndexType::Default) {
                Filter {
                    _as: aerospike_core::query::Filter::geo_within_region(bin_name, region),
                }
            } else {
                Filter {
                    _as: aerospike_core::query::Filter::geo_within_region(bin_name, region)
                        .collection_type(aerospike_core::query::CollectionIndexType::from(cit)),
                }
            }
        }

        #[staticmethod]
        pub fn within_region_by_index(
            index_name: &str,
            region: &str,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            if matches!(cit, CollectionIndexType::Default) {
                Filter {
                    _as: aerospike_core::query::Filter::geo_within_region_by_index(
                        index_name,
                        region,
                    ),
                }
            } else {
                Filter {
                    _as: aerospike_core::query::Filter::geo_within_region_by_index(index_name, region)
                        .collection_type(aerospike_core::query::CollectionIndexType::from(cit)),
                }
            }
        }

        #[staticmethod]
        /// Public API uses (lng, lat) to match GeoJSON [longitude, latitude].
        pub fn within_radius(
            bin_name: &str,
            lng: f64,
            lat: f64,
            radius: f64,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            if matches!(cit, CollectionIndexType::Default) {
                Filter {
                    _as: aerospike_core::query::Filter::geo_within_radius(
                        bin_name,
                        lng,
                        lat,
                        radius,
                    ),
                }
            } else {
                Filter {
                    _as: aerospike_core::query::Filter::geo_within_radius(bin_name,
                        lng,
                        lat,
                        radius).collection_type(aerospike_core::query::CollectionIndexType::from(cit)),
                }
            }
        }

        #[staticmethod]
        pub fn within_radius_by_index(
            index_name: &str,
            lng: f64,
            lat: f64,
            radius: f64,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            if matches!(cit, CollectionIndexType::Default) {
                Filter {
                    _as: aerospike_core::query::Filter::geo_within_radius_by_index(
                        index_name,
                        lng,
                        lat,
                        radius,
                    ),
                }
            } else {
                Filter {
                    _as: aerospike_core::query::Filter::geo_within_radius_by_index(index_name,
                        lng,
                        lat,
                        radius).collection_type(aerospike_core::query::CollectionIndexType::from(cit)),
                }
            }
        }

        #[staticmethod]
        pub fn regions_containing_point(
            bin_name: &str,
            point: &str,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            if matches!(cit, CollectionIndexType::Default) {
                Filter {
                    _as: aerospike_core::query::Filter::geo_contains(bin_name, point),
                }
            } else {
                Filter {
                    _as: aerospike_core::query::Filter::geo_contains(bin_name, point)
                        .collection_type(aerospike_core::query::CollectionIndexType::from(cit)),
                }
            }
        }

        #[staticmethod]
        pub fn regions_containing_point_by_index(
            index_name: &str,
            point: &str,
            cit: Option<&CollectionIndexType>,
        ) -> Self {
            let default = CollectionIndexType::Default;
            let cit = cit.unwrap_or(&default);
            if matches!(cit, CollectionIndexType::Default) {
                Filter {
                    _as: aerospike_core::query::Filter::geo_contains_by_index(
                        index_name,
                        point,
                    ),
                }
            } else {
                Filter {
                    _as: aerospike_core::query::Filter::geo_contains_by_index(index_name, point)
                        .collection_type(aerospike_core::query::CollectionIndexType::from(cit)),
                }
            }
        }
    }
    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Recordset
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Virtual collection of records retrieved through queries and scans. During a query/scan,
    /// multiple threads will retrieve records from the server nodes and put these records on an
    /// internal queue managed by the recordset. The single user thread consumes these records from the
    /// queue.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, 
        name = "Recordset",
        module = "_native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1000))]
    pub struct Recordset {
        pub(crate) _as: Arc<aerospike_core::Recordset>,
        pub(crate) _stream: Arc<Mutex<Option<Pin<Box<RecordStream>>>>>,
        // Some when built from `Client::query` (async); None when built from
        // `Client::query_blocking`. Async iteration / `partition_filter()`
        // route through the bridge for loop-affinity + per-Client runtime;
        // a None bridge surfaces an explicit refusal rather than silently
        // routing through the global runtime.
        pub(crate) bridge: Option<crate::completion::CompletionBridge>,
    }

    impl Clone for Recordset {
        fn clone(&self) -> Self {
            // The bridge holds a Py<...> which needs the GIL to bump
            // refcount; PyO3 from_py_object expects a plain Clone. Drop the
            // bridge on clone — async paths read bridge from the original
            // (via PyRef in `__aiter__`), so this only loses bridge access
            // for from_py_object conversions which don't iterate.
            Recordset {
                _as: self._as.clone(),
                _stream: Arc::new(Mutex::new(None)),
                bridge: None,
            }
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Recordset {
        pub fn close(&self) {
            self._as.close();
        }

        #[getter]
        pub fn get_active(&self) -> bool {
            self._as.is_active()
        }

        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Optional[PartitionFilter]]", imports=("typing", "aerospike_native")))]
        pub fn partition_filter<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
            let bridge = self.bridge.as_ref().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "Cannot await partition_filter() on a Recordset created via \
                     query_blocking. Use query() in an async context.",
                )
            })?;
            let recordset = self._as.clone();

            crate::completion::batched_future_into_py(bridge, py, async move {
                match recordset.partition_filter() {
                    Some(pf) => Ok(Some(PartitionFilter { _as: pf })),
                    None => Ok(None),
                }
            })
        }

        pub fn partition_filter_sync(&self, py: Python<'_>) -> PyResult<Option<PartitionFilter>> {
            // Synchronous counterpart to `partition_filter()` for the blocking
            // query path, where a Recordset created via `query_blocking` has no
            // CompletionBridge to resolve an awaitable on.
            if crate::blocking::in_async_context(py)? {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "Cannot call partition_filter_sync() from within an async \
                     context. Use `await partition_filter()` instead.",
                ));
            }
            let recordset = self._as.clone();
            let pf = py.detach(|| recordset.partition_filter());
            Ok(pf.map(|pf| PartitionFilter { _as: pf }))
        }

        fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __anext__<'a>(&'a mut self, py: Python<'a>) -> PyResult<Py<PyAny>> {
            let bridge = self.bridge.as_ref().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "Cannot async-iterate a Recordset created via query_blocking. \
                     Use sync iteration (`for record in recordset:`) or recreate \
                     the recordset via Client.query() in an async context.",
                )
            })?;
            let recordset = self._as.clone();
            let stream_mutex = self._stream.clone();

            crate::completion::batched_future_into_py(bridge, py, async move {
                // Initialize stream if needed, then poll
                let mut stream_opt = stream_mutex.lock().await;
                if stream_opt.is_none() {
                    *stream_opt = Some(Box::pin(recordset.clone().into_stream()));
                }

                if let Some(ref mut stream) = *stream_opt {
                    use futures::StreamExt;
                    // Return a plain (Send) Rust value; the CompletionBridge's
                    // converter builds the Python `Record` on the drainer/loop
                    // thread. Never `Python::attach` here — this runs on a Tokio
                    // worker, and registering a PyThreadState on a worker
                    // segfaults on free-threaded finalization teardown (see the
                    // invariant in waker.rs and the lazy pattern in errors.rs).
                    // `PyErr::from(RustClientError(..))` is lazy, so the error
                    // arm never attaches on the worker either.
                    match stream.as_mut().next().await {
                        Some(Ok(rec)) => {
                            Ok(Record { _as: rec, cached_bins: None, cached_results: None })
                        }
                        Some(Err(e)) => Err(PyErr::from(RustClientError(e))),
                        None => Err(PyStopAsyncIteration::new_err("Recordset iteration complete")),
                    }
                } else {
                    Err(PyStopAsyncIteration::new_err("Recordset iteration complete"))
                }
            })
            .map(|bound| bound.unbind())
        }

        // Blocking iteration — returned by `Client.query_blocking()`.  Same
        // shape as the async path: drive the underlying stream, but block
        // the Python thread (releasing the GIL via py.detach) instead of
        // returning an awaitable each step.  Raising the standard
        // `StopIteration` ends the loop the Pythonic way.
        fn __iter__(&self) -> Self {
            self.clone()
        }

        fn __next__(&mut self, py: Python<'_>) -> PyResult<Record> {
            // The async-context guard is checked here too, not just in
            // `query_blocking`: a user could legally create the Recordset
            // before entering an async context and then iterate it from
            // inside one.
            if crate::blocking::in_async_context(py)? {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "Cannot iterate a blocking Recordset from within an async \
                     context.  Use `async for record in recordset:` instead.",
                ));
            }

            let recordset = self._as.clone();
            let stream_mutex = self._stream.clone();
            let rt = pyo3_async_runtimes::tokio::get_runtime();

            let result = py.detach(|| {
                crate::blocking::block_on(rt, async move {
                    let mut stream_opt = stream_mutex.lock().await;
                    if stream_opt.is_none() {
                        *stream_opt = Some(Box::pin(recordset.clone().into_stream()));
                    }
                    let stream = stream_opt.as_mut().unwrap();
                    use futures::StreamExt;
                    match stream.as_mut().next().await {
                        Some(Ok(rec)) => Ok(Some(rec)),
                        Some(Err(e)) => Err(PyErr::from(RustClientError(e))),
                        None => Ok(None),
                    }
                })
            })??;

            match result {
                Some(rec) => Ok(Record { _as: rec, cached_bins: None, cached_results: None }),
                None => Err(pyo3::exceptions::PyStopIteration::new_err(())),
            }
        }
    }
