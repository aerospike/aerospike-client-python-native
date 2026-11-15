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

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::IntoPyObjectExt;

use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};

use crate::enums::ResultCode;

////////////////////////////////////////////////////////////////////////////////////////////
//
//  LatencyUnit
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Unit latency histograms are recorded in.
///
/// Microseconds (24 default columns) resolve sub-millisecond work; milliseconds
/// (7 default columns) match the classic column scheme. Changing the unit on a
/// running client discards accumulated latency samples — microsecond and
/// millisecond values cannot share buckets.
#[gen_stub_pyclass_enum(module = "_native")]
#[pyclass(from_py_object, module = "_native")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LatencyUnit {
    #[pyo3(name = "MICROSECONDS")]
    Microseconds,
    #[pyo3(name = "MILLISECONDS")]
    Milliseconds,
}

#[gen_stub_pymethods]
#[pymethods]
impl LatencyUnit {
    fn __richcmp__(&self, other: &LatencyUnit, op: pyo3::class::basic::CompareOp) -> PyResult<bool> {
        match op {
            pyo3::class::basic::CompareOp::Eq => Ok(self == other),
            pyo3::class::basic::CompareOp::Ne => Ok(self != other),
            _ => Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "Only == and != comparisons are supported",
            )),
        }
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    /// The wire/config form of the unit: "us" or "ms".
    fn __str__(&self) -> &'static str {
        aerospike_core::LatencyUnit::from(self).as_str()
    }
}

impl From<&LatencyUnit> for aerospike_core::LatencyUnit {
    fn from(input: &LatencyUnit) -> Self {
        match input {
            LatencyUnit::Microseconds => aerospike_core::LatencyUnit::Microseconds,
            LatencyUnit::Milliseconds => aerospike_core::LatencyUnit::Milliseconds,
        }
    }
}

impl From<&aerospike_core::LatencyUnit> for LatencyUnit {
    fn from(input: &aerospike_core::LatencyUnit) -> Self {
        match input {
            aerospike_core::LatencyUnit::Microseconds => LatencyUnit::Microseconds,
            aerospike_core::LatencyUnit::Milliseconds => LatencyUnit::Milliseconds,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  CommandType
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Canonical per-command metric categories.
///
/// These are the keys of the detailed metrics and per-command histograms on
/// :class:`NodeMetricsSnapshot`. `NONE` marks commands outside every category
/// and has no histogram of its own.
#[gen_stub_pyclass_enum(module = "_native")]
#[pyclass(from_py_object, module = "_native")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandType {
    #[pyo3(name = "NONE")]
    None,
    #[pyo3(name = "GET")]
    Get,
    #[pyo3(name = "GET_HEADER")]
    GetHeader,
    #[pyo3(name = "EXISTS")]
    Exists,
    #[pyo3(name = "PUT")]
    Put,
    #[pyo3(name = "DELETE")]
    Delete,
    #[pyo3(name = "OPERATE")]
    Operate,
    #[pyo3(name = "QUERY")]
    Query,
    #[pyo3(name = "SCAN")]
    Scan,
    #[pyo3(name = "UDF")]
    Udf,
    #[pyo3(name = "BATCH_READ")]
    BatchRead,
    #[pyo3(name = "BATCH_WRITE")]
    BatchWrite,
}

#[gen_stub_pymethods]
#[pymethods]
impl CommandType {
    fn __richcmp__(&self, other: &CommandType, op: pyo3::class::basic::CompareOp) -> PyResult<bool> {
        match op {
            pyo3::class::basic::CompareOp::Eq => Ok(self == other),
            pyo3::class::basic::CompareOp::Ne => Ok(self != other),
            _ => Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "Only == and != comparisons are supported",
            )),
        }
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    /// The name used as a key in serialized detailed metrics (e.g. "GetHeader", "UDF").
    fn __str__(&self) -> &'static str {
        aerospike_core::CommandType::from(self).as_str()
    }
}

impl From<&CommandType> for aerospike_core::CommandType {
    fn from(input: &CommandType) -> Self {
        match input {
            CommandType::None => aerospike_core::CommandType::None,
            CommandType::Get => aerospike_core::CommandType::Get,
            CommandType::GetHeader => aerospike_core::CommandType::GetHeader,
            CommandType::Exists => aerospike_core::CommandType::Exists,
            CommandType::Put => aerospike_core::CommandType::Put,
            CommandType::Delete => aerospike_core::CommandType::Delete,
            CommandType::Operate => aerospike_core::CommandType::Operate,
            CommandType::Query => aerospike_core::CommandType::Query,
            CommandType::Scan => aerospike_core::CommandType::Scan,
            CommandType::Udf => aerospike_core::CommandType::Udf,
            CommandType::BatchRead => aerospike_core::CommandType::BatchRead,
            CommandType::BatchWrite => aerospike_core::CommandType::BatchWrite,
        }
    }
}

impl From<&aerospike_core::CommandType> for CommandType {
    fn from(input: &aerospike_core::CommandType) -> Self {
        match input {
            aerospike_core::CommandType::None => CommandType::None,
            aerospike_core::CommandType::Get => CommandType::Get,
            aerospike_core::CommandType::GetHeader => CommandType::GetHeader,
            aerospike_core::CommandType::Exists => CommandType::Exists,
            aerospike_core::CommandType::Put => CommandType::Put,
            aerospike_core::CommandType::Delete => CommandType::Delete,
            aerospike_core::CommandType::Operate => CommandType::Operate,
            aerospike_core::CommandType::Query => CommandType::Query,
            aerospike_core::CommandType::Scan => CommandType::Scan,
            aerospike_core::CommandType::Udf => CommandType::Udf,
            aerospike_core::CommandType::BatchRead => CommandType::BatchRead,
            aerospike_core::CommandType::BatchWrite => CommandType::BatchWrite,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  Sampler
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Sampling policy for the extended (per-command) metrics.
///
/// A command is sampled when `hash % range < threshold`, so `threshold / range`
/// is the sampled fraction. Counters and gauges are always collected while
/// metrics are enabled; only the per-command histograms and detailed metrics
/// are sampler-gated.
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, module = "_native")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sampler {
    pub(crate) _as: aerospike_core::Sampler,
}

#[gen_stub_pymethods]
#[pymethods]
impl Sampler {
    /// Sample `threshold` out of every `range` commands.
    #[new]
    pub fn new(range: u64, threshold: u64) -> Self {
        Sampler {
            _as: aerospike_core::Sampler::new(range, threshold),
        }
    }

    /// Sample every command (the default).
    #[staticmethod]
    pub fn all() -> Self {
        Sampler {
            _as: aerospike_core::Sampler::all(),
        }
    }

    /// Sample no commands.
    #[staticmethod]
    pub fn never() -> Self {
        Sampler {
            _as: aerospike_core::Sampler::never(),
        }
    }

    /// Sample approximately `p` (0.0-1.0) of commands.
    #[staticmethod]
    pub fn probability(p: f64) -> Self {
        Sampler {
            _as: aerospike_core::Sampler::probability(p),
        }
    }

    #[getter]
    pub fn get_range(&self) -> u64 {
        self._as.range()
    }

    #[getter]
    pub fn get_threshold(&self) -> u64 {
        self._as.threshold()
    }

    fn __repr__(&self) -> String {
        format!("Sampler(range={}, threshold={})", self._as.range(), self._as.threshold())
    }

    fn __richcmp__(&self, other: &Sampler, op: pyo3::class::basic::CompareOp) -> PyResult<bool> {
        match op {
            pyo3::class::basic::CompareOp::Eq => Ok(self._as == other._as),
            pyo3::class::basic::CompareOp::Ne => Ok(self._as != other._as),
            _ => Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "Only == and != comparisons are supported",
            )),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  MetricsPolicy
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Configuration for client metrics collection.
///
/// Defaults are milliseconds with 7 range-layout columns and a shift of 1,
/// sampling every call.
///
/// `MetricsPolicy.micros()` selects microsecond resolution with 24 columns.
/// Re-enabling metrics with a changed latency unit or histogram shape discards
/// the accumulated latency samples.
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, module = "_native")]
#[derive(Debug, Clone)]
pub struct MetricsPolicy {
    pub(crate) _as: aerospike_core::MetricsPolicy,
}

impl Default for MetricsPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl MetricsPolicy {
    #[new]
    pub fn new() -> Self {
        // The cross-SDK default is milliseconds / 7 columns / shift 1, with the
        // operational tier off. Named rather than taken from core's `default()`
        // so the binding's advertised default cannot drift with core's.
        MetricsPolicy {
            _as: aerospike_core::MetricsPolicy::millis(),
        }
    }

    /// Microsecond resolution with 24 range-layout columns.
    #[staticmethod]
    pub fn micros() -> Self {
        MetricsPolicy {
            _as: aerospike_core::MetricsPolicy::micros(),
        }
    }

    /// Millisecond resolution with 7 range-layout columns (the default).
    #[staticmethod]
    pub fn millis() -> Self {
        MetricsPolicy {
            _as: aerospike_core::MetricsPolicy::millis(),
        }
    }


    /// Whether the operational tier is recorded: latency and byte histograms,
    /// result codes, and the command retry/error and connection failure
    /// counters. Off by default, leaving only the always-on gauges -- pool
    /// occupancy, connections opened and closed, tend and node counts.
    #[getter]
    pub fn get_operational(&self) -> bool {
        self._as.operational
    }

    #[setter]
    pub fn set_operational(&mut self, operational: bool) {
        self._as.operational = operational;
    }

    #[getter]
    pub fn get_latency_unit(&self) -> LatencyUnit {
        (&self._as.latency_unit).into()
    }

    #[setter]
    pub fn set_latency_unit(&mut self, latency_unit: LatencyUnit) {
        self._as.latency_unit = (&latency_unit).into();
    }

    #[getter]
    pub fn get_latency_columns(&self) -> usize {
        self._as.latency_columns
    }

    #[setter]
    pub fn set_latency_columns(&mut self, latency_columns: usize) {
        self._as.latency_columns = latency_columns;
    }

    /// Range-layout shift: each boundary after the first two (`<=1`, `>1`)
    /// multiplies by ``2 ** latency_shift``.
    #[getter]
    pub fn get_latency_shift(&self) -> usize {
        self._as.latency_shift as usize
    }

    #[setter]
    pub fn set_latency_shift(&mut self, latency_shift: usize) -> PyResult<()> {
        // A shift of 0 gives a multiplier of 1, which is a degenerate
        // histogram: every boundary equals the one before it.
        if latency_shift < 1 {
            return Err(PyValueError::new_err("latency_shift must be at least 1"));
        }
        self._as.latency_shift = latency_shift as u32;
        Ok(())
    }

    /// Histogram multiplier, always ``2 ** latency_shift``. Read-only: set
    /// :attr:`latency_shift` so the two cannot disagree.
    #[getter]
    pub fn get_latency_base(&self) -> usize {
        1usize << self._as.latency_shift
    }

    /// Static label sets attached to every snapshot (e.g. `[{"team": "billing"}]`).
    #[getter]
    pub fn get_labels(&self) -> Vec<HashMap<String, String>> {
        self._as.labels.entries().to_vec()
    }

    #[setter]
    pub fn set_labels(&mut self, labels: Vec<HashMap<String, String>>) {
        self._as.labels = aerospike_core::Labels::with_pairs(labels);
    }

    #[getter]
    pub fn get_sampler(&self) -> Sampler {
        Sampler { _as: self._as.sampler }
    }

    #[setter]
    pub fn set_sampler(&mut self, sampler: Sampler) {
        self._as.sampler = sampler._as;
    }

    fn __repr__(&self) -> String {
        format!(
            "MetricsPolicy(operational={}, latency_unit={}, latency_columns={}, \
             latency_shift={}, sampler=({}, {}))",
            if self._as.operational { "True" } else { "False" },
            self._as.latency_unit.as_str(),
            self._as.latency_columns,
            self.get_latency_shift(),
            self._as.sampler.range(),
            self._as.sampler.threshold(),
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  Histogram
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Snapshot of one histogram: bucket counts plus min/max/sum/count of the raw
/// values. Latency histograms are in the snapshot's latency unit; byte-size
/// histograms are in bytes regardless of the unit.
#[gen_stub_pyclass(module = "_native")]
#[pyclass(module = "_native")]
#[derive(Debug, Clone)]
pub struct Histogram {
    pub(crate) _as: aerospike_core::metrics::SyncHistogram,
}

#[gen_stub_pymethods]
#[pymethods]
impl Histogram {
    #[getter]
    pub fn get_buckets(&self) -> Vec<u64> {
        self._as.buckets()
    }

    #[getter]
    pub fn get_count(&self) -> u64 {
        self._as.count()
    }

    #[getter]
    pub fn get_min(&self) -> u64 {
        self._as.min()
    }

    #[getter]
    pub fn get_max(&self) -> u64 {
        self._as.max()
    }

    #[getter]
    pub fn get_sum(&self) -> f64 {
        self._as.sum()
    }

    #[getter]
    pub fn get_average(&self) -> f64 {
        self._as.average()
    }

    fn __repr__(&self) -> String {
        format!(
            "Histogram(count={}, min={}, max={}, average={:.2})",
            self._as.count(),
            self._as.min(),
            self._as.max(),
            self._as.average(),
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  CommandMetric
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Detailed per-(namespace, command type) metrics: phase latency histograms
/// plus byte-size histograms.
#[gen_stub_pyclass(module = "_native")]
#[pyclass(module = "_native")]
#[derive(Debug, Clone)]
pub struct CommandMetric {
    pub(crate) _as: aerospike_core::metrics::CommandMetric,
}

#[gen_stub_pymethods]
#[pymethods]
impl CommandMetric {
    /// Connection-acquisition latency (pool hit or new connection).
    #[getter]
    pub fn get_connection_aq(&self) -> Histogram {
        Histogram { _as: self._as.connection_aq.clone() }
    }

    /// Total command latency, including retries.
    #[getter]
    pub fn get_latency(&self) -> Histogram {
        Histogram { _as: self._as.latency.clone() }
    }

    /// Response-parsing latency.
    #[getter]
    pub fn get_parsing(&self) -> Histogram {
        Histogram { _as: self._as.parsing.clone() }
    }

    /// Request sizes in bytes.
    #[getter]
    pub fn get_bytes_sent(&self) -> Histogram {
        Histogram { _as: self._as.bytes_sent.clone() }
    }

    /// Response sizes in bytes.
    #[getter]
    pub fn get_bytes_received(&self) -> Histogram {
        Histogram { _as: self._as.bytes_received.clone() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  NodeMetricsSnapshot
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Accumulated metrics for one node (or the cluster-aggregated view).
///
/// Counter values are cumulative since metrics were enabled;
/// `connections_open` is a point-in-time gauge. Latency histogram buckets are
/// meaningless without `latency_unit`, which is carried on the snapshot.
#[gen_stub_pyclass(module = "_native")]
#[pyclass(module = "_native")]
#[derive(Debug, Clone)]
pub struct NodeMetricsSnapshot {
    pub(crate) _as: aerospike_core::NodeMetricsSnapshot,
}

#[gen_stub_pymethods]
#[pymethods]
impl NodeMetricsSnapshot {
    /// Unit the latency histograms were recorded in.
    #[getter]
    pub fn get_latency_unit(&self) -> LatencyUnit {
        (&self._as.latency_unit).into()
    }

    /// Label sets from the metrics policy.
    #[getter]
    pub fn get_labels(&self) -> Vec<HashMap<String, String>> {
        self._as.labels.entries().to_vec()
    }

    /// Per-command-type latency histogram, or None for `CommandType.NONE`.
    pub fn command_histogram(&self, command_type: CommandType) -> Option<Histogram> {
        self._as
            .command_histogram((&command_type).into())
            .map(|h| Histogram { _as: h.clone() })
    }

    /// Detailed metrics recorded for a (namespace, command type) pair, if any.
    pub fn detailed_metric(&self, namespace: &str, command_type: CommandType) -> Option<CommandMetric> {
        self._as
            .detailed_metric(namespace, (&command_type).into())
            .map(|m| CommandMetric { _as: m.clone() })
    }

    /// Count recorded for a (namespace, command type, result code) triple.
    pub fn result_code_count(&self, namespace: &str, command_type: CommandType, result_code: &ResultCode) -> u64 {
        // Core counts server answers only; a client-side code has no slot.
        result_code.server().map_or(0, |rc| {
            self._as.result_code_count(namespace, (&command_type).into(), rc)
        })
    }

    /// Namespaces that have detailed metrics recorded.
    pub fn detailed_namespaces(&self) -> Vec<String> {
        self._as
            .detailed_namespaces()
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    /// Open-connections gauge (point-in-time, not cumulative).
    #[getter]
    pub fn get_open_connections(&self) -> u64 {
        self._as.open_connections()
    }

    #[getter]
    pub fn get_connections_attempts(&self) -> u64 {
        self._as.counters.connections_attempts
    }

    #[getter]
    pub fn get_connections_successful(&self) -> u64 {
        self._as.counters.connections_successful
    }

    #[getter]
    pub fn get_connections_failed(&self) -> u64 {
        self._as.counters.connections_failed
    }

    #[getter]
    pub fn get_connections_timeout_errors(&self) -> u64 {
        self._as.counters.connections_timeout_errors
    }

    #[getter]
    pub fn get_connections_other_errors(&self) -> u64 {
        self._as.counters.connections_other_errors
    }

    /// Connection attempts that failed during the TLS handshake.
    #[getter]
    pub fn get_connections_tls_errors(&self) -> u64 {
        self._as.counters.connections_tls_errors
    }

    /// Connection attempts that failed authenticating.
    #[getter]
    pub fn get_connections_auth_errors(&self) -> u64 {
        self._as.counters.connections_auth_errors
    }

    #[getter]
    pub fn get_circuit_breaker_hits(&self) -> u64 {
        self._as.counters.circuit_breaker_hits
    }

    #[getter]
    pub fn get_connections_pool_empty(&self) -> u64 {
        self._as.counters.connections_pool_empty
    }

    #[getter]
    pub fn get_connections_pool_overflow(&self) -> u64 {
        self._as.counters.connections_pool_overflow
    }

    #[getter]
    pub fn get_connections_idle_dropped(&self) -> u64 {
        self._as.counters.connections_idle_dropped
    }

    /// Connections checked out of the pool right now (point-in-time gauge).
    #[getter]
    pub fn get_connections_in_use(&self) -> u64 {
        self._as.counters.connections_in_use
    }

    /// Connections sitting idle in the pool right now (point-in-time gauge).
    #[getter]
    pub fn get_connections_in_pool(&self) -> u64 {
        self._as.counters.connections_in_pool
    }

    #[getter]
    pub fn get_connections_closed(&self) -> u64 {
        self._as.counters.connections_closed
    }

    /// Connections closed because the connection itself errored.
    #[getter]
    pub fn get_connections_closed_error(&self) -> u64 {
        self._as.counters.connections_closed_error
    }

    /// Connections closed because their node left the cluster.
    #[getter]
    pub fn get_connections_closed_node_removed(&self) -> u64 {
        self._as.counters.connections_closed_node_removed
    }

    /// Connections currently being recovered after a timeout.
    ///
    /// A point-in-time gauge stamped at snapshot, not a cumulative count: the
    /// `connections_recovered` counter it replaces was never incremented.
    #[getter]
    pub fn get_connections_recovering(&self) -> u64 {
        self._as.counters.connections_recovering
    }

    #[getter]
    pub fn get_tends_total(&self) -> u64 {
        self._as.counters.tends_total
    }

    #[getter]
    pub fn get_tends_successful(&self) -> u64 {
        self._as.counters.tends_successful
    }

    #[getter]
    pub fn get_tends_failed(&self) -> u64 {
        self._as.counters.tends_failed
    }

    #[getter]
    pub fn get_partition_map_updates(&self) -> u64 {
        self._as.counters.partition_map_updates
    }

    #[getter]
    pub fn get_node_added(&self) -> u64 {
        self._as.counters.node_added
    }

    #[getter]
    pub fn get_node_removed(&self) -> u64 {
        self._as.counters.node_removed
    }

    #[getter]
    pub fn get_transaction_retry_count(&self) -> u64 {
        self._as.counters.transaction_retry_count
    }

    #[getter]
    pub fn get_transaction_error_count(&self) -> u64 {
        self._as.counters.transaction_error_count
    }

    /// Errored commands as a fraction of commands attempted, stamped at
    /// snapshot over the interval the snapshot covers.
    #[getter]
    pub fn get_error_rate(&self) -> u64 {
        self._as.counters.error_rate
    }

    /// The full snapshot as a plain dict, using the cross-client-stable
    /// serialized names (e.g. "get-metrics", "detailed-resultcode-counts").
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        serialize_to_py(py, &self._as)
    }

    fn __repr__(&self) -> String {
        format!(
            "NodeMetricsSnapshot(latency_unit={}, open_connections={})",
            self._as.latency_unit.as_str(),
            self._as.open_connections(),
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  ClusterMetrics
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Cluster-wide metrics snapshot: per-node snapshots keyed by host address,
/// a cluster-aggregated snapshot, and cluster-level counters.
#[gen_stub_pyclass(module = "_native")]
#[pyclass(module = "_native")]
#[derive(Debug, Clone)]
pub struct ClusterMetrics {
    pub(crate) _as: aerospike_core::ClusterMetrics,
}

#[gen_stub_pymethods]
#[pymethods]
impl ClusterMetrics {
    /// Per-node snapshots keyed by host address.
    #[getter]
    pub fn get_nodes(&self) -> HashMap<String, NodeMetricsSnapshot> {
        self._as
            .nodes
            .iter()
            .map(|(host, snap)| (host.clone(), NodeMetricsSnapshot { _as: snap.clone() }))
            .collect()
    }

    /// All node snapshots aggregated into one view.
    #[getter]
    pub fn get_cluster_aggregated(&self) -> NodeMetricsSnapshot {
        NodeMetricsSnapshot { _as: self._as.cluster_aggregated.clone() }
    }

    #[getter]
    pub fn get_total_nodes(&self) -> usize {
        self._as.total_nodes
    }

    /// Open connections across the cluster (point-in-time gauge).
    #[getter]
    pub fn get_open_connections(&self) -> u64 {
        self._as.open_connections
    }

    /// Connections checked out across the cluster (point-in-time gauge).
    #[getter]
    pub fn get_connections_in_use(&self) -> u64 {
        self._as.connections_in_use
    }

    /// Connections idle in their pools across the cluster (point-in-time gauge).
    #[getter]
    pub fn get_connections_in_pool(&self) -> u64 {
        self._as.connections_in_pool
    }

    /// Connections queued for recovery after a timeout (point-in-time gauge).
    #[getter]
    pub fn get_recover_queue_size(&self) -> u64 {
        self._as.recover_queue_size
    }

    /// Nodes the tend loop currently considers unusable (point-in-time gauge).
    #[getter]
    pub fn get_nodes_invalid(&self) -> u64 {
        self._as.nodes_invalid
    }

    /// Commands that failed after exhausting max retries (cumulative).
    #[getter]
    pub fn get_exceeded_max_retries(&self) -> u64 {
        self._as.exceeded_max_retries
    }

    /// Commands that failed on total timeout (cumulative).
    #[getter]
    pub fn get_exceeded_total_timeout(&self) -> u64 {
        self._as.exceeded_total_timeout
    }

    /// The full snapshot as a plain dict, using the cross-client-stable
    /// serialized names. Node snapshots appear under their host address;
    /// the aggregate under "cluster-aggregated-metrics".
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        serialize_to_py(py, &self._as)
    }

    fn __repr__(&self) -> String {
        format!(
            "ClusterMetrics(total_nodes={}, open_connections={}, exceeded_max_retries={}, exceeded_total_timeout={})",
            self._as.total_nodes,
            self._as.open_connections,
            self._as.exceeded_max_retries,
            self._as.exceeded_total_timeout,
        )
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  serde -> Python conversion
//
////////////////////////////////////////////////////////////////////////////////////////////

fn serialize_to_py<T: serde::Serialize>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>> {
    let json = serde_json::to_value(value).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("failed to serialize metrics: {e}"))
    })?;
    json_to_py(py, &json)
}

fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => b.into_py_any(py),
        serde_json::Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                u.into_py_any(py)
            } else if let Some(i) = n.as_i64() {
                i.into_py_any(py)
            } else {
                // Non-integral numbers (histogram sums/averages).
                n.as_f64().unwrap_or(f64::NAN).into_py_any(py)
            }
        }
        serde_json::Value::String(s) => s.into_py_any(py),
        serde_json::Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(json_to_py(py, item)?)?;
            }
            list.into_py_any(py)
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, val) in map {
                dict.set_item(key, json_to_py(py, val)?)?;
            }
            dict.into_py_any(py)
        }
    }
}
