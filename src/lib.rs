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

#![deny(warnings)]
#![allow(clippy::new_without_default)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::too_many_arguments)]
extern crate pyo3;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::collections::HashMap;
use indexmap::IndexMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use pyo3::exceptions::{PyException, PyStopAsyncIteration, PyValueError};
use pyo3::exceptions::PyTypeError;
use pyo3::types::{PyDict, PyList, PyTuple};
use pyo3::{prelude::*, IntoPyObjectExt};

use pyo3_async_runtimes::tokio as pyo3_asyncio;
use pyo3_stub_gen::{
    define_stub_info_gatherer, derive::gen_stub_pyclass, derive::gen_stub_pyfunction,
    derive::gen_stub_pymethods,
};

use tokio::sync::Mutex;

use aerospike_core::errors::Error;


mod blocking;
mod completion;
mod waker;
mod enums;
mod errors;
mod runtime;
mod client_runtime;
mod tasks;
mod tls;
mod host;
mod record;
mod cdt;
mod expressions;
mod filter;
mod operations;
mod policies;
mod query_plan;
mod cluster;
mod string_ops;
mod server_error;
mod metrics;

pub use enums::*;
pub use errors::*;
pub use tasks::*;
pub use record::*;
pub use cdt::*;
pub use expressions::*;
pub use filter::*;
pub use operations::*;
pub use policies::*;
pub use query_plan::*;
pub use cluster::*;
pub use server_error::*;
pub use tls::*;
pub use host::*;
pub use string_ops::*;
pub use metrics::*;

define_stub_info_gatherer!(stub_info);

use crate::blocking::run_blocking;
use crate::cdt::ctx_to_vec;
use crate::operations::{
    bins_flag, bins_from_dict, bins_from_dict_list, convert_ops_with_ctx_to_core,
    extract_py_ops_with_ctx,
};

    /**********************************************************************************
     *
     * Client
     *
     **********************************************************************************/
    #[gen_stub_pyfunction(module = "_native")]
    #[pyfunction]
    #[gen_stub(override_return_type(type_repr="typing.Awaitable[Client]", imports=("typing")))]
    pub fn new_client(py: Python, policy: ClientPolicy, seeds: Seeds) -> PyResult<Py<PyAny>> {
        let as_policy = policy._as.clone();
        let seed_string = seeds.to_seed_string();
        let cluster_name = as_policy.cluster_name.clone();
        // Capture the loop the caller is awaiting on; this becomes the bridge's
        // owning loop and every subsequent op on this Client must run on it.
        let locals = pyo3_asyncio::get_current_locals(py)?;
        let owning_loop: Py<PyAny> = locals.event_loop(py).clone().unbind();
        // Per-Client Tokio runtime, opt-in via
        // `ClientPolicy.per_client_runtime_workers`. When set, ops on this
        // Client run on a dedicated runtime, eliminating cross-loop scheduler
        // contention under AsyncPool (which auto-sets this field). Default
        // None / Some(0) preserves the global-runtime behavior.
        let client_rt = match policy.per_client_runtime_workers {
            Some(n) if n >= 1 => Some(Arc::new(
                crate::client_runtime::ClientRuntime::new(n).map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(
                        format!("failed to start per-Client Tokio runtime: {e}"),
                    )
                })?,
            )),
            _ => None,
        };
        let bridge = completion::CompletionBridge::new(py, owning_loop, client_rt)?;
        // When per-Client runtime is in use, the aerospike-core Client (and
        // its TCP sockets) must be constructed on that same runtime so its
        // reactor owns the socket registrations. Routing the construction
        // future through batched_future_into_py (which respects
        // bridge.inner.client_rt) ensures this. Going through the regular
        // pyo3_asyncio::future_into_py would register sockets with the
        // global runtime's reactor while ops ran on the per-Client one,
        // queuing I/O serially behind the global reactor (~5ms latency).
        let bridge_for_construction = bridge.clone();
        Ok(completion::batched_future_into_py(&bridge_for_construction, py, async move {
            log::debug!(target: "aerospike_native", "connecting to {}", seed_string);
            let c = aerospike_core::Client::new(&as_policy, seeds.as_to_hosts())
                .await
                .map_err(|e| PyErr::from(RustClientError(e)))?;

            log::debug!(target: "aerospike_native", "connected to {}", seed_string);
            let res = Client {
                _as: Arc::new(c),
                seeds: seed_string,
                cluster_name,
                bridge: Some(bridge),
            };

            Ok(res)
        })?
        .into())
    }

    // `new_client_blocking` lives in :mod:`blocking` alongside the other
    // shared blocking-flavor infrastructure (``run_blocking`` etc.).

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Txn
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Multi-Record Transaction handle.
    ///
    /// Pass a `Txn` instance to record operations to group them into a single
    /// atomic transaction. Call :meth:`Client.commit` or :meth:`Client.abort`
    /// to finalize the transaction.
    ///
    /// Example::
    ///
    ///     txn = aerospike_native.Txn()
    ///     # use txn in put/get policy.txn field
    ///     status = await client.commit(txn)
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Clone)]
    pub struct Txn {
        pub(crate) _as: Arc<aerospike_core::Txn>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Txn {
        /// Create a new multi-record transaction.
        #[new]
        pub fn new() -> Self {
            Txn { _as: Arc::new(aerospike_core::Txn::new()) }
        }

        /// Unique transaction ID assigned by the client.
        #[getter]
        pub fn id(&self) -> i64 {
            self._as.id()
        }

        /// Current state of the transaction.
        #[getter]
        pub fn state(&self) -> TxnState {
            self._as.state().into()
        }

        /// Force the transaction into a given state.
        ///
        /// Primarily useful for testing error paths that depend on a
        /// non-``OPEN`` transaction (commands must be issued against an
        /// ``OPEN`` transaction). Production code should let
        /// :meth:`Client.commit` / :meth:`Client.abort` drive state
        /// transitions.
        #[setter]
        pub fn set_state(&self, state: TxnState) {
            let core_state = match state {
                TxnState::Open => aerospike_core::TxnState::Open,
                TxnState::Verified => aerospike_core::TxnState::Verified,
                TxnState::Committed => aerospike_core::TxnState::Committed,
                TxnState::Aborted => aerospike_core::TxnState::Aborted,
                TxnState::CommitFailed => aerospike_core::TxnState::CommitFailed,
            };
            self._as.set_state(core_state);
        }

        /// Whether the transaction's outcome is in doubt.
        ///
        /// Set by the client when a commit fails without a definite result:
        /// the server may still roll the transaction forward, so the outcome
        /// is unknown. Recovery is to retry the commit rather than re-run the
        /// transaction, which could apply the same writes twice.
        #[getter]
        pub fn in_doubt(&self) -> bool {
            self._as.in_doubt()
        }

        /// Transaction timeout in seconds. Zero means use the server default.
        #[getter]
        pub fn timeout(&self) -> u32 {
            self._as.timeout().as_secs() as u32
        }

        /// Set the transaction timeout in seconds.
        ///
        /// Must be set before the transaction is shared with a policy or
        /// operation. After the underlying ``Arc<Txn>`` has been cloned into
        /// a policy / builder the timeout is frozen and this raises
        /// :class:`ValueError`. In a transactional session, set the timeout
        /// immediately after entering the ``async with`` block (before the
        /// first ``execute()``).
        #[setter]
        pub fn set_timeout(&mut self, timeout: u32) -> PyResult<()> {
            match Arc::get_mut(&mut self._as) {
                Some(txn) => {
                    txn.set_timeout(std::time::Duration::from_secs(u64::from(timeout)));
                    Ok(())
                }
                None => Err(PyValueError::new_err(
                    "Cannot mutate Txn.timeout after the transaction has been \
                     shared with a policy or operation; set the timeout before \
                     the first operation in the transactional session."
                )),
            }
        }

        /// Namespace in use by this transaction, if one has been set.
        #[getter]
        pub fn namespace(&self) -> Option<String> {
            self._as.namespace()
        }

        fn __repr__(&self) -> String {
            format!("Txn(id={}, state={:?}, namespace={:?})", self._as.id(), self._as.state(), self._as.namespace())
        }
    }

    // Per-slot carrier for `Client._submit_many_read` / `_submit_many_write`.
    // Holds the record (reads), nothing (writes), or the op's exception so
    // one failed key cannot poison its window-mates; converts on the drainer
    // thread to the `Record` object, `None`, or the exception INSTANCE (not
    // raised) so callers can distinguish slots with
    // `isinstance(slot, Exception)`.
    struct SlotOutcome(Result<Option<Record>, PyErr>);

    impl<'py> IntoPyObject<'py> for SlotOutcome {
        type Target = PyAny;
        type Output = Bound<'py, PyAny>;
        type Error = PyErr;

        fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
            match self.0 {
                Ok(Some(record)) => Ok(record.into_pyobject(py)?.into_any()),
                Ok(None) => Ok(py.None().into_bound(py)),
                Err(e) => Ok(e.into_value(py).into_bound(py).into_any()),
            }
        }
    }

    // Whole-window carrier: delivers `(slots, failure_count)`. The count is
    // taken during the one pass that already converts each slot, so a caller
    // that wants to post-process failed slots can skip failure-free windows
    // with a single integer test instead of scanning every slot.
    struct SlotWindow(Vec<SlotOutcome>);

    impl<'py> IntoPyObject<'py> for SlotWindow {
        type Target = PyTuple;
        type Output = Bound<'py, PyTuple>;
        type Error = PyErr;

        fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
            let mut failures: usize = 0;
            let slots = PyList::new(
                py,
                self.0.into_iter().map(|slot| {
                    if slot.0.is_err() {
                        failures += 1;
                    }
                    slot.into_pyobject(py)
                }).collect::<PyResult<Vec<_>>>()?,
            )?;
            PyTuple::new(py, [slots.into_any(), failures.into_bound_py_any(py)?])
        }
    }

    // Deferred-conversion carrier for `Client.exists_legacy`.  Returned from
    // the spawned Tokio task and converted to a `(Key, meta_dict | None)`
    // Python tuple inside pyo3-async-runtimes' single delivery-time
    // `Python::attach`.  This pattern avoids the redundant `Python::attach`
    // that would otherwise run on the Tokio worker thread.
    struct PendingExistsLegacy {
        key: aerospike_core::Key,
        meta_record: Option<aerospike_core::Record>,
    }

    impl<'py> IntoPyObject<'py> for PendingExistsLegacy {
        type Target = PyAny;
        type Output = Bound<'py, PyAny>;
        type Error = PyErr;

        fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
            let key_obj = Py::new(py, Key { _as: self.key })?;
            let meta: Py<PyAny> = if let Some(record) = self.meta_record {
                let meta_dict = pyo3::types::PyDict::new(py);
                meta_dict.set_item("gen", record.generation)?;
                if let Some(ttl) = record.time_to_live() {
                    meta_dict.set_item("ttl", ttl.as_secs() as u32)?;
                } else {
                    meta_dict.set_item("ttl", py.None())?;
                }
                meta_dict.into()
            } else {
                py.None()
            };
            let tuple = pyo3::types::PyTuple::new(py, [key_obj.into_any(), meta])?;
            Ok(tuple.into_any())
        }
    }

    /// Fast PRNG (xoshiro256++ via rand crate) exposed for Python callers
    /// that want sub-100 ns random number generation per call. CPython's
    /// stdlib `random.Random` uses Mersenne Twister at ~700 ns/call —
    /// fine for general use, but a measurable handicap in benchmark hot
    /// loops where every µs counts. Rust core uses `SmallRng` for the
    /// same reason; this exposes the equivalent to Python so benchmark
    /// methodology stays consistent across language layers.
    ///
    /// Not thread-safe — construct one per worker thread / task.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(module = "_native")]
    pub struct FastRng {
        inner: rand::rngs::SmallRng,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl FastRng {
        /// Create a new generator. ``seed=None`` seeds from OS entropy.
        #[new]
        #[pyo3(signature = (seed = None))]
        pub fn new(seed: Option<u64>) -> Self {
            use rand::SeedableRng;
            let inner = match seed {
                Some(s) => rand::rngs::SmallRng::seed_from_u64(s),
                None => rand::rngs::SmallRng::from_entropy(),
            };
            FastRng { inner }
        }

        /// Return a uniformly random integer in ``[lo, hi]`` (inclusive).
        /// ~5 ns per call vs CPython's ~700 ns for ``random.randint``.
        pub fn randint(&mut self, lo: i64, hi: i64) -> i64 {
            use rand::Rng;
            self.inner.gen_range(lo..=hi)
        }

        /// Return a uniformly random integer in ``[0, max)`` (exclusive).
        pub fn gen_below(&mut self, max: u64) -> u64 {
            use rand::Rng;
            self.inner.gen_range(0..max)
        }

        /// Return a uniformly random integer in ``[start, stop)`` (half-open).
        /// Matches CPython's ``random.Random.randrange(start, stop)`` API.
        pub fn randrange(&mut self, start: i64, stop: i64) -> i64 {
            use rand::Rng;
            self.inner.gen_range(start..stop)
        }
    }

    /// Sync-only Aerospike client that owns a per-thread `current_thread`
    /// Tokio runtime. Eliminates the cross-thread worker hop per op that the
    /// shared multi-thread runtime imposes on `Client_blocking()`'s sync path.
    ///
    /// Construct one per Python OS thread. The runtime + Client are tied
    /// to the thread that built them — accessing from a different thread
    /// raises (enforced by `#[pyclass(unsendable)]`). Use `threading.local()`
    /// in Python to manage thread-bound instances.
    ///
    /// Surface mirrors the `Client.*_blocking` methods PSDK's sync path
    /// calls. Each `*_local` method runs on the per-thread runtime via
    /// `block_on`, so completion returns on the SAME thread without any
    /// worker-pool hop.
    /// **Experimental — subject to removal.** Per-thread `current_thread`
    /// Tokio runtime client. Open caveat: cluster-tend multiplication at
    /// high thread counts.
    /// Default users should stick with [`Client`]. Underscore-prefixed name
    /// signals private / unstable status; opt in via PSDK's
    /// `SyncClient(current_thread_runtime=True)`.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(unsendable, module = "_native", name = "_LocalClient")]
    pub struct LocalClient {
        rt: tokio::runtime::Runtime,
        client: Arc<aerospike_core::Client>,
        // See `Client.cluster_name`: configured validation name, captured once.
        cluster_name: Option<String>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl LocalClient {
        #[new]
        pub fn new(policy: ClientPolicy, seeds: Seeds) -> PyResult<Self> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(
                    format!("failed to build current_thread Tokio runtime: {e}")
                ))?;
            let as_policy = policy._as.clone();
            let cluster_name = as_policy.cluster_name.clone();
            let client = crate::blocking::block_on(&rt, async move {
                aerospike_core::Client::new(&as_policy, seeds.as_to_hosts()).await
            })?.map_err(|e| PyErr::from(RustClientError(e)))?;
            Ok(LocalClient {
                rt,
                client: Arc::new(client),
                cluster_name,
            })
        }

        /// Configured cluster name (see :attr:`Client.cluster_name`).
        #[getter]
        pub fn cluster_name(&self) -> Option<String> {
            self.cluster_name.clone()
        }

        /// Server-reported cluster name (see
        /// :attr:`Client.server_cluster_name`).
        #[getter]
        pub fn server_cluster_name(&self) -> Option<String> {
            self.client.server_cluster_name()
        }

        // -- Plain blocking ops (drop-in for Client.*_blocking) -------------

        #[pyo3(signature = (
            key,
            bins=None,
            *,
            policy=None,
            policy_sc=None,
            filter_expression=None,
            txn=None,
        ))]
        pub fn get_blocking<'a>(
            &self,
            key: &Key,
            bins: Option<Vec<String>>,
            policy: Option<ReadPolicy>,
            policy_sc: Option<ReadPolicy>,
            filter_expression: Option<FilterExpression>,
            txn: Option<Txn>,
            py: Python<'a>,
        ) -> PyResult<Record> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let key_as = key._as.clone();
            let client = self.client.clone();
            let raw = py.detach(|| {
                crate::blocking::block_on(&self.rt, async move {
                    let mut policy = match base_sc {
                        Some(sc) => {
                            let is_sc = client.is_strong_consistency(key_as.namespace()).unwrap_or(false);
                            if is_sc { sc } else { base_ap }
                        }
                        None => base_ap,
                    };
                    apply_read_overrides(&mut policy, filter_expression, txn);
                    let res = client
                        .get(&policy, &key_as, bins_flag(bins))
                        .await
                        .map_err(|e| PyErr::from(RustClientError(e)))?;
                    Ok::<_, PyErr>(res)
                })
            })??;
            Ok(Record { _as: raw, cached_bins: None, cached_results: None })
        }

        #[pyo3(signature = (key, bins, *, policy=None, policy_sc=None, txn=None))]
        pub fn put_blocking<'a>(
            &self,
            key: &Key,
            bins: HashMap<String, PythonValue>,
            policy: Option<WritePolicy>,
            policy_sc: Option<WritePolicy>,
            txn: Option<Txn>,
            py: Python<'a>,
        ) -> PyResult<()> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let key_as = key._as.clone();
            let client = self.client.clone();
            let core_bins: Vec<aerospike_core::Bin> = bins
                .into_iter()
                .map(|(name, val)| aerospike_core::Bin::new(name, val.into()))
                .collect();
            py.detach(|| -> PyResult<()> {
                crate::blocking::block_on(&self.rt, async move {
                    let mut policy = match base_sc {
                        Some(sc) => {
                            let is_sc = client.is_strong_consistency(key_as.namespace()).unwrap_or(false);
                            if is_sc { sc } else { base_ap }
                        }
                        None => base_ap,
                    };
                    if let Some(t) = txn {
                        policy.base_policy.txn = Some(t._as);
                    }
                    client.put(&policy, &key_as, &core_bins).await
                        .map_err(|e| PyErr::from(RustClientError(e)))?;
                    Ok(())
                })?
            })
        }

        #[pyo3(signature = (key, *, policy=None))]
        pub fn delete_blocking<'a>(
            &self,
            key: &Key,
            policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<bool> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key_as = key._as.clone();
            let client = self.client.clone();
            py.detach(|| {
                crate::blocking::block_on(&self.rt, async move {
                    client.delete(&policy, &key_as).await
                })
            })?.map_err(|e| PyErr::from(RustClientError(e)))
        }

        #[pyo3(signature = (key, *, policy=None))]
        pub fn touch_blocking<'a>(
            &self,
            key: &Key,
            policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<()> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key_as = key._as.clone();
            let client = self.client.clone();
            py.detach(|| {
                crate::blocking::block_on(&self.rt, async move {
                    client.touch(&policy, &key_as).await
                })
            })?.map_err(|e| PyErr::from(RustClientError(e)))?;
            Ok(())
        }

        #[pyo3(signature = (key, *, policy=None))]
        pub fn exists_blocking<'a>(
            &self,
            key: &Key,
            policy: Option<ReadPolicy>,
            py: Python<'a>,
        ) -> PyResult<bool> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key_as = key._as.clone();
            let client = self.client.clone();
            py.detach(|| {
                crate::blocking::block_on(&self.rt, async move {
                    client.exists(&policy, &key_as).await
                })
            })?.map_err(|e| PyErr::from(RustClientError(e)))
        }

        #[pyo3(signature = (
            key,
            operations,
            *,
            policy=None,
            policy_sc=None,
            record_exists_action=None,
            expiration=None,
            generation=None,
            durable_delete=None,
            filter_expression=None,
            txn=None,
        ))]
        pub fn operate_blocking<'a>(
            &self,
            key: &Key,
            operations: Vec<Py<PyAny>>,
            policy: Option<WritePolicy>,
            policy_sc: Option<WritePolicy>,
            record_exists_action: Option<RecordExistsAction>,
            expiration: Option<Expiration>,
            generation: Option<u32>,
            durable_delete: Option<bool>,
            filter_expression: Option<FilterExpression>,
            txn: Option<Txn>,
            py: Python<'a>,
        ) -> PyResult<Record> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let key_as = key._as.clone();
            let client = self.client.clone();
            let rust_ops = extract_py_ops_with_ctx(py, &operations)?;
            let raw = py.detach(|| -> PyResult<aerospike_core::Record> {
                crate::blocking::block_on(&self.rt, async move {
                    let mut policy = match base_sc {
                        Some(sc) => {
                            let is_sc = client.is_strong_consistency(key_as.namespace()).unwrap_or(false);
                            if is_sc { sc } else { base_ap }
                        }
                        None => base_ap,
                    };
                    apply_write_overrides(
                        &mut policy,
                        record_exists_action,
                        expiration,
                        generation,
                        durable_delete,
                        filter_expression,
                        txn,
                    );
                    let (core_ops, _) = convert_ops_with_ctx_to_core(&rust_ops, false)?;
                    client.operate(&policy, &key_as, &core_ops).await
                        .map_err(|e| PyErr::from(RustClientError(e)))
                })?
            })?;
            Ok(Record { _as: raw, cached_bins: None, cached_results: None })
        }

        /// Returns whether ``namespace`` is configured for strong
        /// consistency on the cluster.  See :meth:`Client.is_strong_consistency`
        /// for semantics.
        pub fn is_strong_consistency(&self, namespace: &str) -> Option<bool> {
            self.client.is_strong_consistency(namespace)
        }

        // -- Info (needed by PSDK SyncSession namespace-mode resolver) ------

        pub fn info_blocking(
            &self,
            command: String,
            py: Python<'_>,
        ) -> PyResult<IndexMap<String, String>> {
            let client = self.client.clone();
            py.detach(|| {
                crate::blocking::block_on(&self.rt, async move {
                    let node = client.random_node()
                        .map_err(|e| PyErr::from(RustClientError(e)))?;
                    let policy = aerospike_core::AdminPolicy::default();
                    node.info(&policy, &[&command]).await
                        .map_err(|e| PyErr::from(RustClientError(e)))
                })
            })?
        }

        // -- Metrics (collection lives in the core; these are synchronous) --

        /// Enable metrics collection. Defaults to :class:`MetricsPolicy`'s
        /// milliseconds/7-column scheme recording every command when no
        /// policy is given.
        #[pyo3(signature = (policy = None))]
        pub fn enable_metrics(&self, policy: Option<MetricsPolicy>) {
            self.client
                .enable_metrics(policy.unwrap_or_default()._as);
        }

        /// Disable metrics collection. Accumulated data is retained.
        pub fn disable_metrics(&self) {
            self.client.disable_metrics();
        }

        /// Whether metrics collection is currently enabled.
        pub fn metrics_enabled(&self) -> bool {
            self.client.metrics_enabled()
        }

        /// Snapshot cluster metrics. Values are cumulative since metrics were
        /// enabled (gauges are point-in-time); drains and aggregates per-node
        /// state, so poll at an export interval rather than per operation.
        pub fn metrics(&self, py: Python<'_>) -> ClusterMetrics {
            let client = self.client.clone();
            py.detach(move || ClusterMetrics { _as: client.metrics() })
        }
    }

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass)]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Clone)]
    pub struct Client {
        _as: Arc<aerospike_core::Client>,
        seeds: String,
        // Configured cluster-name from ClientPolicy (the validation name), or
        // None when cluster-name validation was not requested. Captured once at
        // construction so callers can tag diagnostics without a per-call clone
        // of the whole policy through the core.
        cluster_name: Option<String>,
        // None for clients created via `new_client_blocking` — async methods
        // require this to be Some; `require_bridge()` enforces it with a clear
        // PyRuntimeError instead of panicking.
        bridge: Option<completion::CompletionBridge>,
    }

    // Boxed receiver of the per-node-completion BatchRecord stream, bridged
    // from core's `batch_foreach` exactly-once row hook. Rows arrive in
    // completion order; a terminal `Err` item carries the batch's whole-node
    // failure and raises from whichever yield reaches it.
    type BoxedBatchStream = Pin<Box<
        dyn futures::Stream<
            Item = Result<(usize, aerospike_core::BatchRecord), aerospike_core::errors::Error>,
        > + Send,
    >>;

    /// Moves the result row out of a completed batch operation.
    fn take_batch_record(op: aerospike_core::BatchOperation) -> aerospike_core::BatchRecord {
        op.into_batch_record()
    }

    /// Converts a whole-batch failure into the Python error contract. The
    /// batch left its per-key results in place on the ops; once any row
    /// carries an outcome those results are part of the story and ride the
    /// error as `BatchFailedError.records`. A failure
    /// before anything was dispatched — every key unroutable, for example —
    /// leaves every row untouched and surfaces the cause raw. Rows are moved,
    /// not cloned; the ops are spent either way once the call has failed.
    fn batch_failed_with_rows(
        source: aerospike_core::errors::Error,
        ops: Vec<aerospike_core::BatchOperation>,
    ) -> PyErr {
        if ops.iter().any(|op| op.result_code().is_some()) {
            let records = ops.into_iter().map(take_batch_record).collect();
            crate::errors::batch_failed_error(records, source)
        } else {
            PyErr::from(RustClientError(source))
        }
    }

    /// Bridges core's push-style `batch_foreach` back into the lazily-pulled
    /// stream shape the Python surface exposes. Returns the driver future to
    /// spawn (per-Client runtime for the async entry, global runtime for the
    /// blocking one) and the receiver the stream wraps.
    ///
    /// The channel is deliberately unbounded: the row count is bounded by the
    /// input size, so the worst case buffers exactly the full result set —
    /// what the non-streaming entry points materialize anyway — while the
    /// per-row hook never blocks a node reader on a slow Python consumer.
    /// Dropping the receiver makes the next send fail; the `false` the hook
    /// then returns aborts the batch's remaining work.
    fn batch_foreach_bridge(
        client: std::sync::Arc<aerospike_core::Client>,
        policy: aerospike_core::BatchPolicy,
        ops: Vec<aerospike_core::BatchOperation>,
    ) -> (impl std::future::Future<Output = ()> + Send + 'static, BoxedBatchStream) {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let driver = async move {
            let hook_tx = tx.clone();
            let mut ops = ops;
            let result = client
                .batch_foreach(&policy, &mut ops, move |idx, row| {
                    // Clone in the sync arm so the returned future is 'static;
                    // the row must also stay on the op for core's in-place
                    // contract, so a copy here is the price of streaming.
                    let keep_going = hook_tx.unbounded_send(Ok((idx, row.clone()))).is_ok();
                    std::future::ready(keep_going)
                })
                .await;
            if let Err(e) = result {
                let _ = tx.unbounded_send(Err(e));
            }
            // `tx` drops here, closing the channel: the consumer sees the end
            // of the stream after the last row (or the terminal error).
        };
        (driver, Box::pin(rx) as BoxedBatchStream)
    }

    /// Async/sync iterator over a streaming batch result.
    ///
    /// Items arrive in **completion order** (the node that responds first
    /// yields first), not input order — each item is a `(idx, BatchRecord)`
    /// tuple carrying the position of the originating op in the input list.
    /// Per-key errors land on the `BatchRecord` (`result_code`); cluster-level
    /// errors raise from `__anext__` / `__next__`. The stream ends with
    /// `StopAsyncIteration` / `StopIteration` once every input op has yielded.
    ///
    /// **Threading**: the receiver is held behind a single Mutex and
    /// `__anext__` / `__next__` hold the lock across the await/poll. This
    /// is intentional — the design assumes a single consumer driving the
    /// stream end-to-end. Fan-out consumers would serialize at the lock.
    ///
    /// **Bridge routing**: an async-constructed stream
    /// (`Client.batch_stream`) carries the originating Client's
    /// `CompletionBridge`, so each `__anext__` future is loop-affinity-
    /// checked and spawned on the same per-Client Tokio runtime as the
    /// Client itself — preserving AsyncPool's per-Client runtime isolation
    /// invariant for streaming. A sync-constructed stream
    /// (`Client.batch_stream_blocking`) has no bridge and rejects
    /// `__anext__` with a clear error: use sync iteration on those, or
    /// recreate the stream via `batch_stream` in an async context.
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(
        name = "BatchRecordStream",
        module = "_native",
        subclass,
    )]
    pub struct BatchRecordStream {
        inner: Arc<Mutex<Option<BoxedBatchStream>>>,
        // Some when built from the async path (`Client::batch_stream`);
        // None when built from `Client::batch_stream_blocking`.
        bridge: Option<completion::CompletionBridge>,
        // Latched by `close()` so iteration terminates even when the eager
        // receiver-drop can't take the lock (a yield future holds it).
        // `Arc` so `__anext__` can clone it into the per-yield future and
        // observe the latch from inside the async block, matching the path
        // the `None` receiver takes to `StopAsyncIteration`.
        closed: Arc<AtomicBool>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl BatchRecordStream {
        fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        #[gen_stub(override_return_type(
            type_repr="typing.Awaitable[typing.Tuple[builtins.int, BatchRecord]]",
            imports=("typing", "aerospike_native"),
        ))]
        fn __anext__<'a>(&'a mut self, py: Python<'a>) -> PyResult<Py<PyAny>> {
            // Route through the originating Client's CompletionBridge so
            // each yield respects loop-affinity and the per-Client Tokio
            // runtime (AsyncPool isolation). A sync-built stream has no
            // bridge — surface that as an explicit refusal rather than
            // silently routing through the global runtime.
            let bridge = self.bridge.as_ref().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "Cannot async-iterate a BatchRecordStream created via \
                     batch_stream_blocking. Use sync iteration \
                     (`for (idx, br) in stream:`) or recreate the stream \
                     via Client.batch_stream() in an async context.",
                )
            })?;
            let inner = self.inner.clone();
            let closed = self.closed.clone();
            completion::batched_future_into_py(bridge, py, async move {
                use futures::StreamExt;
                if closed.load(Ordering::Acquire) {
                    return Err(PyStopAsyncIteration::new_err(()));
                }
                let mut guard = inner.lock().await;
                let next = match guard.as_mut() {
                    Some(stream) => stream.as_mut().next().await,
                    None => None,
                };
                // Return a plain (Send) Rust value; the CompletionBridge's
                // converter builds the Python tuple on the drainer/loop thread.
                // Never `Python::attach` here — this runs on a Tokio worker, and
                // registering a PyThreadState on a worker segfaults on
                // free-threaded finalization teardown (see the invariant in
                // waker.rs and the lazy pattern in errors.rs).
                match next {
                    Some(Ok((idx, br))) => Ok((idx, BatchRecord { _as: br })),
                    Some(Err(e)) => Err(PyErr::from(RustClientError(e))),
                    None => Err(PyStopAsyncIteration::new_err(())),
                }
            })
            .map(|bound| bound.unbind())
        }

        fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
            slf
        }

        fn __next__(&mut self, py: Python<'_>) -> PyResult<(usize, BatchRecord)> {
            if self.closed.load(Ordering::Acquire) {
                return Err(pyo3::exceptions::PyStopIteration::new_err(()));
            }
            // Match Recordset's guard: blocking iteration from inside an
            // async event loop would block the loop. Tell the user clearly.
            if blocking::in_async_context(py)? {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "Cannot iterate a blocking BatchRecordStream from within an \
                     async context. Use `async for (idx, br) in stream:` instead.",
                ));
            }
            let inner = self.inner.clone();
            let rt = pyo3_async_runtimes::tokio::get_runtime();
            let next = py.detach(|| {
                blocking::block_on(rt, async move {
                    use futures::StreamExt;
                    let mut guard = inner.lock().await;
                    match guard.as_mut() {
                        Some(stream) => stream.as_mut().next().await,
                        None => None,
                    }
                })
            })?;
            match next {
                Some(Ok((idx, br))) => Ok((idx, BatchRecord { _as: br })),
                Some(Err(e)) => Err(PyErr::from(RustClientError(e))),
                None => Err(pyo3::exceptions::PyStopIteration::new_err(())),
            }
        }

        /// Release the batch stream early.
        ///
        /// Latches the stream closed so any subsequent iteration terminates
        /// with ``StopAsyncIteration`` / ``StopIteration``, and eagerly drops
        /// the receiver together with any buffered-but-unconsumed results —
        /// deterministically, rather than waiting for garbage collection.
        ///
        /// **Scope**: dropping the receiver also aborts the batch's remaining
        /// work — the first row delivered after the drop fails to send, and
        /// the delivery hook's refusal tears down the running node groups.
        /// A node response already being parsed finishes on its own.
        /// Idempotent, and safe to call from either an async or a blocking
        /// context.
        ///
        /// If a yield is in progress at the instant of the call (an internal
        /// lock is held), the eager receiver-drop is skipped and cleanup falls
        /// back to normal drop semantics once that yield settles; the closed
        /// latch still takes effect immediately.
        fn close(&self) {
            self.closed.store(true, Ordering::Release);
            // Non-blocking: never stall the caller's thread (asyncio loop
            // thread for the async stream) waiting on an in-flight yield.
            if let Ok(mut guard) = self.inner.try_lock() {
                *guard = None;
            }
        }
    }

    // Local enum mirroring the ExtractedOp shape used by `batch` /
    // `batch_blocking`. Kept private here so the streaming entries can share
    // the same GIL-held extract pass without exposing the type at module scope.
    #[derive(Clone)]
    enum ExtractedBatchOp {
        Read {
            key: aerospike_core::Key,
            policy: aerospike_core::BatchReadPolicy,
            bins: Option<Vec<String>>,
            ops: Vec<OpWithCtx>,
        },
        Write {
            key: aerospike_core::Key,
            policy: aerospike_core::BatchWritePolicy,
            ops: Vec<OpWithCtx>,
        },
        Delete {
            key: aerospike_core::Key,
            policy: aerospike_core::BatchDeletePolicy,
        },
        Udf {
            key: aerospike_core::Key,
            policy: aerospike_core::BatchUdfPolicy,
            udf_name: String,
            function_name: String,
            args: Option<Vec<aerospike_core::Value>>,
        },
    }

    fn extract_batch_ops_py(
        py: Python<'_>,
        ops: &[Py<PyAny>],
    ) -> PyResult<Vec<ExtractedBatchOp>> {
        let mut out = Vec::with_capacity(ops.len());
        for op_obj in ops {
            if let Ok(read_op) = op_obj.extract::<PyRef<BatchReadOp>>(py) {
                out.push(ExtractedBatchOp::Read {
                    key: read_op.key.clone(),
                    policy: read_op.policy.clone(),
                    bins: read_op.bins.clone(),
                    ops: read_op.ops.clone(),
                });
            } else if let Ok(write_op) = op_obj.extract::<PyRef<BatchWriteOp>>(py) {
                out.push(ExtractedBatchOp::Write {
                    key: write_op.key.clone(),
                    policy: write_op.policy.clone(),
                    ops: write_op.ops.clone(),
                });
            } else if let Ok(delete_op) = op_obj.extract::<PyRef<BatchDeleteOp>>(py) {
                out.push(ExtractedBatchOp::Delete {
                    key: delete_op.key.clone(),
                    policy: delete_op.policy.clone(),
                });
            } else if let Ok(udf_op) = op_obj.extract::<PyRef<BatchUDFOp>>(py) {
                out.push(ExtractedBatchOp::Udf {
                    key: udf_op.key.clone(),
                    policy: udf_op.policy.clone(),
                    udf_name: udf_op.udf_name.clone(),
                    function_name: udf_op.function_name.clone(),
                    args: udf_op.args.clone(),
                });
            } else {
                return Err(PyTypeError::new_err(
                    "Each op must be a BatchReadOp, BatchWriteOp, BatchDeleteOp, or BatchUDFOp",
                ));
            }
        }
        Ok(out)
    }

    fn build_batch_operations(
        extracted: &[ExtractedBatchOp],
    ) -> PyResult<Vec<aerospike_core::BatchOperation>> {
        use aerospike_core::BatchOperation;
        let mut batch_ops = Vec::with_capacity(extracted.len());
        for ext in extracted {
            match ext {
                ExtractedBatchOp::Read { key, policy, bins, ops } if ops.is_empty() => {
                    batch_ops.push(
                        BatchOperation::read(policy, key.clone(), bins_flag(bins.clone())),
                    );
                }
                ExtractedBatchOp::Read { key, policy, bins: _, ops } => {
                    let (core_ops, _) = convert_ops_with_ctx_to_core(ops, false)?;
                    batch_ops.push(BatchOperation::read_ops(policy, key.clone(), core_ops));
                }
                ExtractedBatchOp::Write { key, policy, ops } => {
                    let (core_ops, _) = convert_ops_with_ctx_to_core(ops, false)?;
                    batch_ops.push(BatchOperation::write(policy, key.clone(), core_ops));
                }
                ExtractedBatchOp::Delete { key, policy } => {
                    batch_ops.push(BatchOperation::delete(policy, key.clone()));
                }
                ExtractedBatchOp::Udf { key, policy, udf_name, function_name, args } => {
                    batch_ops.push(BatchOperation::udf(
                        policy, key.clone(), udf_name, function_name, args.clone().unwrap_or_default(),
                    ));
                }
            }
        }
        Ok(batch_ops)
    }

    // Apply per-call overrides to a cloned ReadPolicy in place.
    fn apply_read_overrides(
        policy: &mut aerospike_core::ReadPolicy,
        filter_expression: Option<FilterExpression>,
        txn: Option<Txn>,
    ) {
        if let Some(fe) = filter_expression {
            policy.base_policy.filter_expression = Some(fe._as);
        }
        if let Some(t) = txn {
            policy.base_policy.txn = Some(t._as);
        }
    }

    // Apply per-call overrides to a cloned WritePolicy in place.
    fn apply_write_overrides(
        policy: &mut aerospike_core::WritePolicy,
        record_exists_action: Option<RecordExistsAction>,
        expiration: Option<Expiration>,
        generation: Option<u32>,
        durable_delete: Option<bool>,
        filter_expression: Option<FilterExpression>,
        txn: Option<Txn>,
    ) {
        if let Some(rea) = record_exists_action {
            policy.record_exists_action = match rea {
                RecordExistsAction::Update => aerospike_core::RecordExistsAction::Update,
                RecordExistsAction::UpdateOnly => aerospike_core::RecordExistsAction::UpdateOnly,
                RecordExistsAction::Replace => aerospike_core::RecordExistsAction::Replace,
                RecordExistsAction::ReplaceOnly => aerospike_core::RecordExistsAction::ReplaceOnly,
                RecordExistsAction::CreateOnly => aerospike_core::RecordExistsAction::CreateOnly,
            };
        }
        if let Some(g) = generation {
            policy.generation = g;
            policy.generation_policy = aerospike_core::GenerationPolicy::ExpectGenEqual;
        }
        if let Some(e) = expiration {
            policy.expiration = (&e).into();
        }
        if let Some(dd) = durable_delete {
            policy.durable_delete = dd;
        }
        if let Some(fe) = filter_expression {
            policy.base_policy.filter_expression = Some(fe._as);
        }
        if let Some(t) = txn {
            policy.base_policy.txn = Some(t._as);
        }
    }

    // Helper function to check if a key exists (internal use, shared by exists() and exists_legacy())
    impl Client {
        async fn exists_internal(
            client: Arc<aerospike_core::Client>,
            policy: aerospike_core::ReadPolicy,
            key: aerospike_core::Key,
        ) -> Result<bool, Error> {
            client.exists(&policy, &key).await
        }

        fn require_bridge(&self) -> PyResult<&completion::CompletionBridge> {
            self.bridge.as_ref().ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "This client was created with new_client_blocking() and cannot \
                     be used for async operations. Create an async client with \
                     new_client() instead.",
                )
            })
        }
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Client {
        #[new]
        pub fn new() -> PyResult<Self> {
            // This is a placeholder constructor - actual initialization should be done via new_client function
            Err(PyException::new_err("Use new_client() function to create a Client instance"))
        }

        pub fn seeds(&self) -> &str {
            &self.seeds
        }

        /// Returns this client library's own version string
        /// (e.g. ``"0.6.0-alpha.6"``).
        ///
        /// This is the version of the installed native client extension you are
        /// running — the same identifier stamped into the wire user-agent — not
        /// the embedded engine core. The value is baked in at build time, so no
        /// cluster connection is required; call it directly on the class without
        /// a live instance. For the embedded engine version, see
        /// :meth:`core_version`.
        ///
        /// Example::
        ///
        ///     from aerospike_native import Client
        ///     print(Client.client_version())
        ///
        /// Returns:
        ///     str: This client library's version.
        #[staticmethod]
        pub fn client_version() -> &'static str {
            env!("CARGO_PKG_VERSION")
        }

        /// Returns the embedded engine core version string
        /// (e.g. ``"3.0.0-alpha.1"``).
        ///
        /// This is the version of the underlying core the client is built
        /// against — useful for support and diagnostics. It is independent of
        /// this client library's own version (see :meth:`client_version`) and,
        /// like it, is baked in at build time and needs no live instance.
        ///
        /// Example::
        ///
        ///     from aerospike_native import Client
        ///     print(Client.core_version())
        ///
        /// Returns:
        ///     str: The embedded engine core version.
        #[staticmethod]
        pub fn core_version() -> &'static str {
            aerospike_core::Client::client_version()
        }

        /// Configured cluster name from the ``ClientPolicy`` used to build this
        /// client, or ``None`` when cluster-name validation was not requested.
        ///
        /// This is the client-side *expected* name (set via cluster-name
        /// validation), not a server-reported value; it reads a cached field
        /// with no network or lock cost.
        #[getter]
        pub fn cluster_name(&self) -> Option<String> {
            self.cluster_name.clone()
        }

        /// Cluster name as the servers report it, or ``None`` when they are
        /// configured without one.
        ///
        /// Unlike :attr:`cluster_name` this does not require the client to
        /// have asked for cluster-name validation -- it is read off the node
        /// list, so it is available on any connected client. Setting a
        /// validation name only rejects nodes that disagree; it does not make
        /// the name readable, which is what this getter is for.
        #[getter]
        pub fn server_cluster_name(&self) -> Option<String> {
            self._as.server_cluster_name()
        }

        /// Returns whether ``namespace`` is configured for strong
        /// consistency on the cluster.
        ///
        /// Reads from the in-memory partition map (no network I/O). PSDK's
        /// :class:`Session` uses this internally; user code rarely needs it.
        ///
        /// Returns:
        ///     ``Some(True)``: SC namespace.
        ///     ``Some(False)``: AP namespace.
        ///     ``None``: namespace not in the partition map (unknown
        ///     namespace, or partition map not yet populated for this cluster).
        pub fn is_strong_consistency(&self, namespace: &str) -> Option<bool> {
            self._as.is_strong_consistency(namespace)
        }

        /// Closes the connection to the Aerospike cluster.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        pub fn close<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();

            // NB: close() does NOT tear down the pipe-wake reader. A closed
            // client is still usable (e.g. `is_connected()` returning False),
            // and those calls still deliver completions through the reader —
            // removing it here would leave them writing to an unwatched pipe
            // and hang. The reader is released when the owning loop closes
            // (asyncio drops readers on `loop.close()`), matching the
            // `call_soon_threadsafe` path, which likewise relies on the loop
            // staying alive after client close.
            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .close()
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(None::<bool>)
            })
        }

        /// Returns true if the client is connected to any cluster nodes.
        ///
        /// Reads the tended node list (no network I/O), so it is a plain
        /// method callable from both async and sync code.
        pub fn is_connected(&self) -> bool {
            self._as.is_connected()
        }

        // ===================================================================
        // Blocking variants: each releases the GIL during the Tokio block_on
        // and raises PyRuntimeError if called from within a running asyncio
        // event loop.
        // ===================================================================

        /// Synchronously close the connection to the Aerospike cluster.
        pub fn close_blocking(&self, py: Python<'_>) -> PyResult<()> {
            let client = self._as.clone();
            run_blocking(py, async move {
                client.close().await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously write record bin(s).
        ///
        /// When `policy_sc` is provided, PNC resolves the key's namespace
        /// mode (AP vs SC) at op time via the cluster accessor and picks
        /// the matching base policy. The `txn` override is applied after
        /// the mode pick on a cloned policy so the caller's cached policies
        /// stay untouched across concurrent ops.
        #[pyo3(signature = (key, bins, *, policy=None, policy_sc=None, txn=None))]
        pub fn put_blocking(
            &self,
            key: &Key,
            bins: &Bound<'_, PyDict>,
            policy: Option<WritePolicy>,
            policy_sc: Option<WritePolicy>,
            txn: Option<Txn>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let key_as = key._as.clone();
            let client = self._as.clone();

            let bin_vec = bins_from_dict(bins)?;

            run_blocking(py, async move {
                let mut policy = match base_sc {
                    Some(sc) => {
                        let is_sc = client.is_strong_consistency(key_as.namespace()).unwrap_or(false);
                        if is_sc { sc } else { base_ap }
                    }
                    None => base_ap,
                };
                if let Some(t) = txn {
                    policy.base_policy.txn = Some(t._as);
                }
                client.put(&policy, &key_as, &bin_vec).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously read a record for the specified key.
        ///
        /// When `policy_sc` is provided, PNC resolves the namespace mode at
        /// op time and picks AP vs SC. `filter_expression` / `txn` are
        /// applied after the mode pick on a cloned policy so the caller's
        /// cached policies stay untouched.
        #[pyo3(signature = (
            key,
            bins=None,
            *,
            policy=None,
            policy_sc=None,
            filter_expression=None,
            txn=None,
        ))]
        pub fn get_blocking(
            &self,
            key: &Key,
            bins: Option<Vec<String>>,
            policy: Option<ReadPolicy>,
            policy_sc: Option<ReadPolicy>,
            filter_expression: Option<FilterExpression>,
            txn: Option<Txn>,
            py: Python<'_>,
        ) -> PyResult<Record> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let key_as = key._as.clone();
            let client = self._as.clone();
            let raw = run_blocking(py, async move {
                let mut policy = match base_sc {
                    Some(sc) => {
                        let is_sc = client.is_strong_consistency(key_as.namespace()).unwrap_or(false);
                        if is_sc { sc } else { base_ap }
                    }
                    None => base_ap,
                };
                apply_read_overrides(&mut policy, filter_expression, txn);
                let res = client
                    .get(&policy, &key_as, bins_flag(bins))
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(res)
            })?;
            Ok(Record { _as: raw, cached_bins: None, cached_results: None })
        }

        /// Synchronously delete a record for the specified key.
        ///
        /// Returns ``True`` if the record existed on the server before the
        /// delete, ``False`` otherwise.
        #[pyo3(signature = (key, *, policy=None))]
        pub fn delete_blocking(
            &self,
            key: &Key,
            policy: Option<WritePolicy>,
            py: Python<'_>,
        ) -> PyResult<bool> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();
            run_blocking(py, async move {
                client.delete(&policy, &key).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously commit a multi-record transaction.
        ///
        /// Same outcomes as :meth:`commit`: success returns a
        /// :class:`CommitStatus`; an abandoned roll-forward raises
        /// :exc:`aerospike_native.exceptions.CommitFailedError`. The optional
        /// ``verify_policy`` / ``roll_policy`` configure the verify and
        /// roll-forward phases; ``None`` applies the default policies.
        #[pyo3(signature = (txn, *, verify_policy=None, roll_policy=None))]
        pub fn commit_blocking(
            &self,
            txn: &Txn,
            verify_policy: Option<TxnVerifyPolicy>,
            roll_policy: Option<TxnRollPolicy>,
            py: Python<'_>,
        ) -> PyResult<CommitStatus> {
            let client = self._as.clone();
            let txn_arc = txn._as.clone();
            // Route through the explicit-policy entry only when a caller
            // supplied one, so the no-policy call keeps core's defaults.
            let policies = match (verify_policy, roll_policy) {
                (None, None) => None,
                (verify, roll) => Some((
                    verify.map(|p| p._as).unwrap_or_default(),
                    roll.map(|p| p._as).unwrap_or_default(),
                )),
            };
            let status = run_blocking(py, async move {
                match &policies {
                    Some((verify, roll)) => {
                        client.commit_with_policies(verify, roll, &txn_arc).await
                    }
                    None => client.commit(&txn_arc).await,
                }
                .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(CommitStatus::from(status))
        }

        /// Synchronously abort a multi-record transaction.
        ///
        /// The optional ``roll_policy`` configures the roll-back phase;
        /// ``None`` applies the default policy.
        #[pyo3(signature = (txn, *, roll_policy=None))]
        pub fn abort_blocking(
            &self,
            txn: &Txn,
            roll_policy: Option<TxnRollPolicy>,
            py: Python<'_>,
        ) -> PyResult<AbortStatus> {
            let client = self._as.clone();
            let txn_arc = txn._as.clone();
            let roll_policy = roll_policy.map(|p| p._as);
            let status = run_blocking(py, async move {
                match &roll_policy {
                    Some(roll) => client.abort_with_policy(roll, &txn_arc).await,
                    None => client.abort(&txn_arc).await,
                }
                .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(AbortStatus::from(status))
        }


        /// Synchronously add integer bin values.
        #[pyo3(signature = (key, bins, *, policy=None))]
        pub fn add_blocking(
            &self,
            key: &Key,
            bins: HashMap<String, PythonValue>,
            policy: Option<WritePolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();
            let bins: Vec<aerospike_core::Bin> = bins.into_iter()
                .map(|(name, val)| aerospike_core::Bin::new(name, val.into()))
                .collect();
            run_blocking(py, async move {
                client.add(&policy, &key, &bins).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously append string bin values.
        #[pyo3(signature = (key, bins, *, policy=None))]
        pub fn append_blocking(
            &self,
            key: &Key,
            bins: HashMap<String, PythonValue>,
            policy: Option<WritePolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();
            let bins: Vec<aerospike_core::Bin> = bins.into_iter()
                .map(|(name, val)| aerospike_core::Bin::new(name, val.into()))
                .collect();
            run_blocking(py, async move {
                client.append(&policy, &key, &bins).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously prepend string bin values.
        #[pyo3(signature = (key, bins, *, policy=None))]
        pub fn prepend_blocking(
            &self,
            key: &Key,
            bins: HashMap<String, PythonValue>,
            policy: Option<WritePolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();
            let bins: Vec<aerospike_core::Bin> = bins.into_iter()
                .map(|(name, val)| aerospike_core::Bin::new(name, val.into()))
                .collect();
            run_blocking(py, async move {
                client.prepend(&policy, &key, &bins).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously reset record TTL.
        #[pyo3(signature = (key, *, policy=None))]
        pub fn touch_blocking(
            &self,
            key: &Key,
            policy: Option<WritePolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();
            run_blocking(py, async move {
                client.touch(&policy, &key).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously check whether a record exists.
        #[pyo3(signature = (key, *, policy=None))]
        pub fn exists_blocking(
            &self,
            key: &Key,
            policy: Option<ReadPolicy>,
            py: Python<'_>,
        ) -> PyResult<bool> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();
            run_blocking(py, async move {
                Self::exists_internal(client, policy, key).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously execute multiple operations atomically on a single record.
        ///
        /// When `policy_sc` is provided, PNC resolves the namespace mode at
        /// op time and picks AP vs SC. The full override set is applied
        /// after the mode pick on a cloned policy so the caller's cached
        /// policies stay untouched across concurrent ops.
        #[pyo3(signature = (
            key,
            operations,
            *,
            policy=None,
            policy_sc=None,
            record_exists_action=None,
            expiration=None,
            generation=None,
            durable_delete=None,
            filter_expression=None,
            txn=None,
        ))]
        pub fn operate_blocking(
            &self,
            key: &Key,
            operations: Vec<Py<PyAny>>,
            policy: Option<WritePolicy>,
            policy_sc: Option<WritePolicy>,
            record_exists_action: Option<RecordExistsAction>,
            expiration: Option<Expiration>,
            generation: Option<u32>,
            durable_delete: Option<bool>,
            filter_expression: Option<FilterExpression>,
            txn: Option<Txn>,
            py: Python<'_>,
        ) -> PyResult<Record> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let key_as = key._as.clone();
            let client = self._as.clone();
            let rust_ops = extract_py_ops_with_ctx(py, &operations)?;
            let raw = run_blocking(py, async move {
                let mut policy = match base_sc {
                    Some(sc) => {
                        let is_sc = client.is_strong_consistency(key_as.namespace()).unwrap_or(false);
                        if is_sc { sc } else { base_ap }
                    }
                    None => base_ap,
                };
                apply_write_overrides(
                    &mut policy,
                    record_exists_action,
                    expiration,
                    generation,
                    durable_delete,
                    filter_expression,
                    txn,
                );
                let (core_ops, _) = convert_ops_with_ctx_to_core(&rust_ops, false)?;
                client.operate(&policy, &key_as, &core_ops).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(Record { _as: raw, cached_bins: None, cached_results: None })
        }

        /// Synchronously execute a registered UDF on a single record.
        #[pyo3(signature = (key, server_path, function_name, args=None, *, policy=None))]
        pub fn execute_udf_blocking(
            &self,
            key: &Key,
            server_path: String,
            function_name: String,
            args: Option<Vec<PythonValue>>,
            policy: Option<WritePolicy>,
            py: Python<'_>,
        ) -> PyResult<Option<PythonValue>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();
            let core_args: Option<Vec<aerospike_core::Value>> =
                args.map(|v| v.into_iter().map(|pv| pv.into()).collect());
            let raw = run_blocking(py, async move {
                let core_args_ref = core_args.as_deref().unwrap_or(&[]);
                client.execute_udf(&policy, &key, &server_path, &function_name, core_args_ref).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(raw.map(PythonValue::from))
        }

        /// Synchronously execute a query and return a Recordset that supports
        /// `for record in recordset:` iteration via `__iter__`/`__next__`.
        #[pyo3(signature = (statement, partition_filter, *, policy=None))]
        pub fn query_blocking(
            &self,
            statement: &Statement,
            partition_filter: PartitionFilter,
            policy: Option<QueryPolicy>,
            py: Python<'_>,
        ) -> PyResult<Recordset> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let stmt = statement.clone()._as;
            let raw = run_blocking(py, async move {
                client.query(&policy, partition_filter._as, stmt).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(Recordset {
                _as: raw,
                _stream: Arc::new(Mutex::new(None)),
                bridge: None,
            })
        }

        /// Synchronously run phase 1 (explain) of server-led query selection.
        ///
        /// Callers must verify :meth:`Version.supports_query_selection` before use;
        /// this method sends field ``44`` regardless of server version.
        ///
        /// Args:
        ///     ael: Filter expression in Aerospike Expression Language; sent on
        ///         field ``44`` and compiled server-side during explain.
        ///     index_name_hint: Field ``21`` soft index-name hint. Becomes strict
        ///         when ``HARD_HINT`` is set in ``explain_where_flags`` — explain then
        ///         fails unless that exact index is selected.
        ///     explain_where_flags: Tier-D field ``44`` bits —
        ///         ``QueryWhereFlags.EXPLAIN`` plus optional ``REQUIRE_INDEX`` /
        ///         ``HARD_HINT``. Omit for default explain.
        ///
        /// A plan with :attr:`QueryPlan.selection` ``FILTERED_OUT`` must not be
        /// passed to :meth:`query_with_plan_blocking` (raises ``FilteredOut``).
        #[pyo3(signature = (namespace, ael, *, set_name=None, index_name_hint=None, explain_where_flags=None, policy=None))]
        pub fn query_explain_blocking(
            &self,
            namespace: String,
            ael: String,
            set_name: Option<String>,
            index_name_hint: Option<String>,
            explain_where_flags: Option<u8>,
            policy: Option<QueryPolicy>,
            py: Python<'_>,
        ) -> PyResult<QueryPlan> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let set_ref = set_name.as_deref();
            let hint_ref = index_name_hint.as_deref();
            let where_flags = explain_where_flags.unwrap_or(aerospike_core::FLAG_EXPLAIN);
            let plan = run_blocking(py, async move {
                client
                    .query_explain(
                        &policy,
                        &namespace,
                        set_ref,
                        &ael,
                        hint_ref,
                        explain_where_flags,
                    )
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            QueryPlan::from_core(plan, where_flags)
        }

        /// Synchronously execute a partitioned query using a server query plan
        /// from :meth:`query_explain_blocking`.
        ///
        /// The index filter is taken from ``plan``; do not set
        /// :attr:`Statement.filters`. ``statement`` must match ``plan`` namespace
        /// and set; a mismatch raises :exc:`ValueError`.
        ///
        /// Args:
        ///     statement: Bins to return; must match the plan namespace/set.
        ///     partition_filter: Partition scope for the query.
        ///     plan: Plan from :meth:`query_explain_blocking`.
        ///     policy: Optional query policy.
        ///
        /// Raises:
        ///     ValueError: If the plan's namespace/set does not match
        ///         ``statement`` (validated client-side before any server round trip),
        ///         or if ``statement`` already has filters.
        ///     FilteredOut: If ``plan.selection`` is ``FILTERED_OUT``.
        #[pyo3(signature = (statement, partition_filter, plan, *, policy=None))]
        pub fn query_with_plan_blocking(
            &self,
            statement: &Statement,
            partition_filter: PartitionFilter,
            plan: &QueryPlan,
            policy: Option<QueryPolicy>,
            py: Python<'_>,
        ) -> PyResult<Recordset> {
            validate_plan_matches_statement(&statement._as, &plan._as)?;
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let stmt = statement._as.clone();
            let core_plan = plan._as.clone();
            let raw = run_blocking(py, async move {
                client
                    .query_with_plan(&policy, partition_filter._as, stmt, core_plan)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(Recordset {
                _as: raw,
                _stream: Arc::new(Mutex::new(None)),
                bridge: None,
            })
        }

        /// Synchronously execute a background query that performs ops on each matching record.
        #[pyo3(signature = (statement, operations, *, write_policy=None))]
        pub fn query_operate_blocking(
            &self,
            statement: &Statement,
            operations: Vec<Py<PyAny>>,
            write_policy: Option<WritePolicy>,
            py: Python<'_>,
        ) -> PyResult<ExecuteTask> {
            let policy = write_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let mut core_statement = statement._as.clone();
            let rust_ops = extract_py_ops_with_ctx(py, &operations)?;
            let (core_ops, _) = convert_ops_with_ctx_to_core(&rust_ops, false)?;
            core_statement.set_operations(core_ops);
            let raw = run_blocking(py, async move {
                client.query_operate(&policy, core_statement).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(ExecuteTask { _as: raw, bridge: None })
        }

        /// Synchronously apply a UDF to records matching the statement (background).
        #[pyo3(signature = (statement, package_name, function_name, args=None, *, write_policy=None))]
        pub fn query_execute_udf_blocking(
            &self,
            statement: &Statement,
            package_name: String,
            function_name: String,
            args: Option<Vec<PythonValue>>,
            write_policy: Option<WritePolicy>,
            py: Python<'_>,
        ) -> PyResult<ExecuteTask> {
            let policy = write_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let mut core_statement = statement._as.clone();
            let rust_args = args.map(|a| a.into_iter().map(|v| v.into())
                .collect::<Vec<aerospike_core::Value>>());
            core_statement.set_aggregate_function(&package_name, &function_name, rust_args.as_deref().unwrap_or(&[]));
            let raw = run_blocking(py, async move {
                let args_ref = rust_args.as_deref().unwrap_or(&[]);
                client.query_execute_udf(&policy, core_statement, &package_name, &function_name, args_ref).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(ExecuteTask { _as: raw, bridge: None })
        }

        /// Synchronously register a UDF module from in-memory bytes.
        #[pyo3(signature = (udf_body, server_path, language, *, policy = None))]
        pub fn register_udf_blocking(
            &self,
            udf_body: Vec<u8>,
            server_path: String,
            language: UDFLang,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<RegisterTask> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let lang: aerospike_core::UdfLang = language.into();
            let raw = run_blocking(py, async move {
                client.register_udf(&admin_policy, &udf_body, &server_path, lang).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(RegisterTask { _as: raw, bridge: None })
        }

        /// Synchronously register a UDF module from a local file path.
        #[pyo3(signature = (client_path, server_path, language, *, policy = None))]
        pub fn register_udf_from_file_blocking(
            &self,
            client_path: String,
            server_path: String,
            language: UDFLang,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<RegisterTask> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let lang: aerospike_core::UdfLang = language.into();
            let raw = run_blocking(py, async move {
                client.register_udf_from_file(&admin_policy, &client_path, &server_path, lang).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(RegisterTask { _as: raw, bridge: None })
        }

        /// Synchronously remove a registered UDF module.
        #[pyo3(signature = (server_path, *, policy = None))]
        pub fn remove_udf_blocking(
            &self,
            server_path: String,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<UdfRemoveTask> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let raw = run_blocking(py, async move {
                client.remove_udf(&admin_policy, &server_path).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(UdfRemoveTask { _as: raw, bridge: None })
        }

        /// Synchronously truncate records in a namespace/set.
        #[pyo3(signature = (namespace, set_name, before_nanos = None, *, policy = None))]
        pub fn truncate_blocking(
            &self,
            namespace: String,
            set_name: String,
            before_nanos: Option<i64>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let before_nanos = before_nanos.unwrap_or_default();
            run_blocking(py, async move {
                client.truncate(&admin_policy, &namespace, &set_name, before_nanos).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously create a secondary index on a bin. Returns an
        /// :class:`IndexTask`; call :meth:`IndexTask.wait_till_complete_blocking`
        /// before querying through the index.
        #[pyo3(signature = (namespace, set_name, bin_name, index_name, index_type, cit = None, ctx = None, *, policy = None))]
        pub fn create_index_blocking(
            &self,
            namespace: String,
            set_name: String,
            bin_name: String,
            index_name: String,
            index_type: IndexType,
            cit: Option<CollectionIndexType>,
            ctx: Option<Vec<CTX>>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<IndexTask> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let cit = (&cit.unwrap_or(CollectionIndexType::Default)).into();
            let index_type = (&index_type).into();
            let ctx_core = ctx.map(|c| ctx_to_vec(&c));
            let raw = run_blocking(py, async move {
                client.create_index_on_bin(
                    &admin_policy, &namespace, &set_name, &bin_name,
                    &index_name, index_type, cit, ctx_core.as_deref(),
                ).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(IndexTask { _as: raw, bridge: None })
        }

        /// Synchronously create a set index (record presence per set; no bin,
        /// type, context or expression). Returns an :class:`IndexTask`; call
        /// :meth:`IndexTask.wait_till_complete_blocking` before relying on it.
        #[pyo3(signature = (namespace, set_name, index_name, *, policy = None))]
        pub fn create_set_index_blocking(
            &self,
            namespace: String,
            set_name: String,
            index_name: String,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<IndexTask> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();
            let raw = run_blocking(py, async move {
                client
                    .create_set_index(&admin_policy, &namespace, &set_name, &index_name)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(IndexTask { _as: raw, bridge: None })
        }

        /// Synchronously drop a secondary index.
        #[pyo3(signature = (namespace, set_name, index_name, *, policy = None))]
        pub fn drop_index_blocking(
            &self,
            namespace: String,
            set_name: String,
            index_name: String,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<DropIndexTask> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let raw = run_blocking(py, async move {
                client.drop_index(&admin_policy, &namespace, &set_name, &index_name).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(DropIndexTask { _as: raw, bridge: None })
        }

        /// Synchronously execute an info command on a random cluster node.
        pub fn info_blocking(
            &self,
            command: String,
            py: Python<'_>,
        ) -> PyResult<IndexMap<String, String>> {
            let client = self._as.clone();
            run_blocking(py, async move {
                let node = client.random_node()
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                let policy = aerospike_core::AdminPolicy::default();
                node.info(&policy, &[&command]).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously execute an info command on all cluster nodes.
        pub fn info_on_all_nodes_blocking(
            &self,
            command: String,
            py: Python<'_>,
        ) -> PyResult<HashMap<String, IndexMap<String, String>>> {
            let client = self._as.clone();
            run_blocking(py, async move {
                let nodes = client.nodes();
                let mut results: HashMap<String, IndexMap<String, String>> = HashMap::new();
                let policy = aerospike_core::AdminPolicy::default();
                for node in nodes {
                    let response = node.info(&policy, &[&command]).await
                        .map_err(|e| PyErr::from(RustClientError(e)))?;
                    results.insert(node.name().to_string(), response);
                }
                Ok(results)
            })
        }

        // -- Security / admin (Group 5) --

        /// Synchronously create a new user.
        #[pyo3(signature = (user, password, roles, *, policy = None))]
        pub fn create_user_blocking(
            &self,
            user: String,
            password: String,
            roles: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            run_blocking(py, async move {
                let roles: Vec<&str> = roles.iter().map(|r| &**r).collect();
                client.create_user(&admin_policy, &user, &password, &roles).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously create a PKI user (TLS-cert auth).
        #[pyo3(signature = (user, roles, *, policy = None))]
        pub fn create_pki_user_blocking(
            &self,
            user: String,
            roles: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            run_blocking(py, async move {
                let roles: Vec<&str> = roles.iter().map(|r| &**r).collect();
                client.create_pki_user(&admin_policy, &user, &roles).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously remove a user.
        #[pyo3(signature = (user, *, policy = None))]
        pub fn drop_user_blocking(
            &self,
            user: String,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            run_blocking(py, async move {
                client.drop_user(&admin_policy, &user).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously change a user's password.
        #[pyo3(signature = (user, password, *, policy = None))]
        pub fn change_password_blocking(
            &self,
            user: String,
            password: String,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            run_blocking(py, async move {
                client.change_password(&admin_policy, &user, &password).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously grant roles to a user.
        #[pyo3(signature = (user, roles, *, policy = None))]
        pub fn grant_roles_blocking(
            &self,
            user: String,
            roles: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            run_blocking(py, async move {
                let roles: Vec<&str> = roles.iter().map(|r| &**r).collect();
                client.grant_roles(&admin_policy, &user, &roles).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously revoke roles from a user.
        #[pyo3(signature = (user, roles, *, policy = None))]
        pub fn revoke_roles_blocking(
            &self,
            user: String,
            roles: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            run_blocking(py, async move {
                let roles: Vec<&str> = roles.iter().map(|r| &**r).collect();
                client.revoke_roles(&admin_policy, &user, &roles).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously query users and their roles.
        #[pyo3(signature = (user = None, *, policy = None))]
        pub fn query_users_blocking(
            &self,
            user: Option<String>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<User>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let raw = run_blocking(py, async move {
                let user_ref = user.as_deref();
                client.query_users(&admin_policy, user_ref).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(raw.into_iter().map(|u| User { _as: u }).collect())
        }

        /// Synchronously query roles and their privileges.
        #[pyo3(signature = (role = None, *, policy = None))]
        pub fn query_roles_blocking(
            &self,
            role: Option<String>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<Role>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let raw = run_blocking(py, async move {
                let role_ref = role.as_deref();
                client.query_roles(&admin_policy, role_ref).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(raw.into_iter().map(|r| Role { _as: r }).collect())
        }

        /// Synchronously create a user-defined role.
        #[pyo3(signature = (role_name, privileges, allowlist, read_quota, write_quota, *, policy = None))]
        pub fn create_role_blocking(
            &self,
            role_name: String,
            privileges: Vec<Privilege>,
            allowlist: Vec<String>,
            read_quota: u32,
            write_quota: u32,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let core_privileges: Vec<aerospike_core::Privilege> =
                privileges.iter().map(|r| r._as.clone()).collect();
            run_blocking(py, async move {
                let allowlist: Vec<&str> = allowlist.iter().map(|al| &**al).collect();
                client.create_role(
                    &admin_policy, &role_name, &core_privileges, &allowlist,
                    read_quota, write_quota,
                ).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously remove a user-defined role.
        #[pyo3(signature = (role_name, *, policy = None))]
        pub fn drop_role_blocking(
            &self,
            role_name: String,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            run_blocking(py, async move {
                client.drop_role(&admin_policy, &role_name).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously grant privileges to a role.
        #[pyo3(signature = (role_name, privileges, *, policy = None))]
        pub fn grant_privileges_blocking(
            &self,
            role_name: String,
            privileges: Vec<Privilege>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let core_privileges: Vec<aerospike_core::Privilege> =
                privileges.iter().map(|p| p._as.clone()).collect();
            run_blocking(py, async move {
                client.grant_privileges(&admin_policy, &role_name, &core_privileges).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously revoke privileges from a role.
        #[pyo3(signature = (role_name, privileges, *, policy = None))]
        pub fn revoke_privileges_blocking(
            &self,
            role_name: String,
            privileges: Vec<Privilege>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let core_privileges: Vec<aerospike_core::Privilege> =
                privileges.iter().map(|p| p._as.clone()).collect();
            run_blocking(py, async move {
                client.revoke_privileges(&admin_policy, &role_name, &core_privileges).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously set IP allowlist for a role.
        #[pyo3(signature = (role_name, allowlist, *, policy = None))]
        pub fn set_allowlist_blocking(
            &self,
            role_name: String,
            allowlist: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            run_blocking(py, async move {
                let allowlist: Vec<&str> = allowlist.iter().map(|al| &**al).collect();
                client.set_allowlist(&admin_policy, &role_name, &allowlist).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously set per-second quotas for a role.
        #[pyo3(signature = (role_name, read_quota, write_quota, *, policy = None))]
        pub fn set_quotas_blocking(
            &self,
            role_name: String,
            read_quota: u32,
            write_quota: u32,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            run_blocking(py, async move {
                client.set_quotas(&admin_policy, &role_name, read_quota, write_quota).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        /// Synchronously create a secondary index using an expression.
        #[pyo3(signature = (namespace, set_name, index_name, index_type, expression, cit = None, *, policy = None))]
        pub fn create_index_using_expression_blocking(
            &self,
            namespace: String,
            set_name: String,
            index_name: String,
            index_type: IndexType,
            expression: &FilterExpression,
            cit: Option<CollectionIndexType>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<IndexTask> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let expr = expression._as.clone();
            let cit = (&cit.unwrap_or(CollectionIndexType::Default)).into();
            let index_type = (&index_type).into();
            let raw = run_blocking(py, async move {
                client.create_index_using_expression(
                    &admin_policy, &namespace, &set_name,
                    &index_name, index_type, cit, &expr,
                ).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })?;
            Ok(IndexTask { _as: raw, bridge: None })
        }

        /// Synchronously set the XDR filter for a datacenter / namespace.
        #[pyo3(signature = (datacenter, namespace, filter_expression = None, *, policy = None))]
        pub fn set_xdr_filter_blocking(
            &self,
            datacenter: String,
            namespace: String,
            filter_expression: Option<FilterExpression>,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as)
                .unwrap_or_default();
            let expr = filter_expression.clone();
            run_blocking(py, async move {
                client.set_xdr_filter(
                    &admin_policy, &datacenter, &namespace,
                    expr.as_ref().map(|e| &e._as),
                ).await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        // -- Batch blocking variants (Group 2) --

        /// Synchronously read multiple records by key in one batch.
        #[pyo3(signature = (keys, bins=None, *, batch_policy=None, read_policy=None))]
        pub fn batch_read_blocking(
            &self,
            keys: Vec<PyRef<Key>>,
            bins: Option<Vec<String>>,
            batch_policy: Option<&BatchPolicy>,
            read_policy: Option<&BatchReadPolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<BatchRecord>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let read_policy = read_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let raw = run_blocking(py, async move {
                use aerospike_core::BatchOperation;
                let bf = bins_flag(bins);
                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    batch_ops.push(BatchOperation::read(&read_policy, key, bf.clone()));
                }
                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                Ok(batch_ops)
            })?;
            Ok(raw.into_iter().map(|op| BatchRecord { _as: take_batch_record(op) }).collect())
        }

        /// Synchronously write multiple records by key in one batch.
        #[pyo3(signature = (keys, bins_list, *, batch_policy=None, write_policy=None))]
        pub fn batch_write_blocking(
            &self,
            keys: Vec<PyRef<Key>>,
            bins_list: Vec<Py<PyAny>>,
            batch_policy: Option<&BatchPolicy>,
            write_policy: Option<&BatchWritePolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<BatchRecord>> {
            if keys.len() != bins_list.len() {
                return Err(PyValueError::new_err(
                    "keys and bins_list must have the same length",
                ));
            }
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let write_policy = write_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let bins_vecs = bins_from_dict_list(py, &bins_list)?;
            let raw = run_blocking(py, async move {
                use aerospike_core::BatchOperation;
                use aerospike_core::operations;
                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for (key, bins) in rust_keys.into_iter().zip(bins_vecs.iter()) {
                    let ops: Vec<aerospike_core::operations::Operation> = bins
                        .iter()
                        .map(operations::put)
                        .collect();
                    batch_ops.push(BatchOperation::write(&write_policy, key, ops));
                }
                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                Ok(batch_ops)
            })?;
            Ok(raw.into_iter().map(|op| BatchRecord { _as: take_batch_record(op) }).collect())
        }

        /// Synchronously perform per-key ops on multiple records in one batch.
        #[pyo3(signature = (keys, operations_list, *, batch_policy=None, write_policy=None))]
        pub fn batch_operate_blocking(
            &self,
            keys: Vec<PyRef<Key>>,
            operations_list: Vec<Vec<Py<PyAny>>>,
            batch_policy: Option<&BatchPolicy>,
            write_policy: Option<&BatchWritePolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<BatchRecord>> {
            if keys.len() != operations_list.len() {
                return Err(PyValueError::new_err(
                    "keys and operations_list must have the same length",
                ));
            }
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let write_policy = write_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let mut rust_ops_list = Vec::with_capacity(operations_list.len());
            for operations in operations_list {
                rust_ops_list.push(extract_py_ops_with_ctx(py, &operations)?);
            }
            let raw = run_blocking(py, async move {
                use aerospike_core::BatchOperation;
                let read_policy = aerospike_core::BatchReadPolicy::default();
                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for (key, ops) in rust_keys.into_iter().zip(rust_ops_list) {
                    let (core_ops, has_write_op) = convert_ops_with_ctx_to_core(&ops, true)?;
                    let batch_op = if has_write_op {
                        BatchOperation::write(&write_policy, key, core_ops)
                    } else {
                        BatchOperation::read_ops(&read_policy, key, core_ops)
                    };
                    batch_ops.push(batch_op);
                }
                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                Ok(batch_ops)
            })?;
            Ok(raw.into_iter().map(|op| BatchRecord { _as: take_batch_record(op) }).collect())
        }

        /// Synchronously delete multiple records by key in one batch.
        #[pyo3(signature = (keys, *, batch_policy=None, delete_policy=None))]
        pub fn batch_delete_blocking(
            &self,
            keys: Vec<PyRef<Key>>,
            batch_policy: Option<&BatchPolicy>,
            delete_policy: Option<&BatchDeletePolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<BatchRecord>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let delete_policy = delete_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let raw = run_blocking(py, async move {
                use aerospike_core::BatchOperation;
                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    batch_ops.push(BatchOperation::delete(&delete_policy, key));
                }
                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                Ok(batch_ops)
            })?;
            Ok(raw.into_iter().map(|op| BatchRecord { _as: take_batch_record(op) }).collect())
        }

        /// Synchronously check existence of multiple keys in one batch.
        #[pyo3(signature = (keys, *, batch_policy=None, read_policy=None))]
        pub fn batch_exists_blocking(
            &self,
            keys: Vec<PyRef<Key>>,
            batch_policy: Option<&BatchPolicy>,
            read_policy: Option<&BatchReadPolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<bool>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let read_policy = read_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let raw = run_blocking(py, async move {
                use aerospike_core::BatchOperation;
                use aerospike_core::Bins;
                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    batch_ops.push(BatchOperation::read(&read_policy, key, Bins::None));
                }
                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                Ok(batch_ops)
            })?;
            Ok(raw.iter().map(|op| op.record().is_some()).collect())
        }

        /// Synchronously read multiple record headers (metadata only) in one batch.
        #[pyo3(signature = (keys, *, batch_policy=None, read_policy=None))]
        pub fn batch_get_header_blocking(
            &self,
            keys: Vec<PyRef<Key>>,
            batch_policy: Option<&BatchPolicy>,
            read_policy: Option<&BatchReadPolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<Option<Record>>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let read_policy = read_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let raw = run_blocking(py, async move {
                use aerospike_core::BatchOperation;
                use aerospike_core::Bins;
                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    batch_ops.push(BatchOperation::read(&read_policy, key, Bins::None));
                }
                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                Ok(batch_ops)
            })?;
            Ok(raw.into_iter()
                .map(|op| {
                    take_batch_record(op)
                        .record
                        .map(|r| Record { _as: r, cached_bins: None, cached_results: None })
                })
                .collect())
        }

        /// Synchronously apply a UDF to multiple keys in one batch.
        #[pyo3(signature = (keys, udf_name, function_name, args, *, batch_policy=None, udf_policy=None))]
        pub fn batch_apply_blocking(
            &self,
            keys: Vec<PyRef<Key>>,
            udf_name: String,
            function_name: String,
            args: Option<Vec<PythonValue>>,
            batch_policy: Option<&BatchPolicy>,
            udf_policy: Option<&BatchUDFPolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<BatchRecord>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let udf_policy = udf_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let rust_args = args.map(|a| a.into_iter().map(|v| v.into())
                .collect::<Vec<aerospike_core::Value>>());
            let raw = run_blocking(py, async move {
                use aerospike_core::BatchOperation;
                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    let rust_args_owned = rust_args.clone().unwrap_or_default();
                    batch_ops.push(BatchOperation::udf(
                        &udf_policy, key, &udf_name, &function_name, rust_args_owned,
                    ));
                }
                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                Ok(batch_ops)
            })?;
            Ok(raw.into_iter().map(|op| BatchRecord { _as: take_batch_record(op) }).collect())
        }

        /// Synchronously execute a mixed batch of read/write/delete ops.
        #[pyo3(signature = (ops, *, batch_policy=None))]
        pub fn batch_blocking(
            &self,
            ops: Vec<Py<PyAny>>,
            batch_policy: Option<&BatchPolicy>,
            py: Python<'_>,
        ) -> PyResult<Vec<BatchRecord>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();

            // Use the same ExtractedOp shape as the async `batch` — kept local
            // so the two implementations stay in sync without exposing the type.
            #[derive(Clone)]
            enum ExtractedOp {
                Read {
                    key: aerospike_core::Key,
                    policy: aerospike_core::BatchReadPolicy,
                    bins: Option<Vec<String>>,
                    ops: Vec<OpWithCtx>,
                },
                Write {
                    key: aerospike_core::Key,
                    policy: aerospike_core::BatchWritePolicy,
                    ops: Vec<OpWithCtx>,
                },
                Delete {
                    key: aerospike_core::Key,
                    policy: aerospike_core::BatchDeletePolicy,
                },
                Udf {
                    key: aerospike_core::Key,
                    policy: aerospike_core::BatchUdfPolicy,
                    udf_name: String,
                    function_name: String,
                    args: Option<Vec<aerospike_core::Value>>,
                },
            }

            let mut extracted: Vec<ExtractedOp> = Vec::with_capacity(ops.len());
            for op_obj in &ops {
                if let Ok(read_op) = op_obj.extract::<PyRef<BatchReadOp>>(py) {
                    extracted.push(ExtractedOp::Read {
                        key: read_op.key.clone(),
                        policy: read_op.policy.clone(),
                        bins: read_op.bins.clone(),
                        ops: read_op.ops.clone(),
                    });
                } else if let Ok(write_op) = op_obj.extract::<PyRef<BatchWriteOp>>(py) {
                    extracted.push(ExtractedOp::Write {
                        key: write_op.key.clone(),
                        policy: write_op.policy.clone(),
                        ops: write_op.ops.clone(),
                    });
                } else if let Ok(delete_op) = op_obj.extract::<PyRef<BatchDeleteOp>>(py) {
                    extracted.push(ExtractedOp::Delete {
                        key: delete_op.key.clone(),
                        policy: delete_op.policy.clone(),
                    });
                } else if let Ok(udf_op) = op_obj.extract::<PyRef<BatchUDFOp>>(py) {
                    extracted.push(ExtractedOp::Udf {
                        key: udf_op.key.clone(),
                        policy: udf_op.policy.clone(),
                        udf_name: udf_op.udf_name.clone(),
                        function_name: udf_op.function_name.clone(),
                        args: udf_op.args.clone(),
                    });
                } else {
                    return Err(PyTypeError::new_err(
                        "Each op must be a BatchReadOp, BatchWriteOp, BatchDeleteOp, or BatchUDFOp",
                    ));
                }
            }

            let raw = run_blocking(py, async move {
                use aerospike_core::BatchOperation;
                let mut batch_ops = Vec::with_capacity(extracted.len());
                for ext in &extracted {
                    match ext {
                        ExtractedOp::Read { key, policy, bins, ops } if ops.is_empty() => {
                            batch_ops.push(
                                BatchOperation::read(policy, key.clone(), bins_flag(bins.clone()))
                            );
                        }
                        ExtractedOp::Read { key, policy, bins: _, ops } => {
                            let (core_ops, _has_write) = convert_ops_with_ctx_to_core(ops, false)?;
                            batch_ops.push(
                                BatchOperation::read_ops(policy, key.clone(), core_ops)
                            );
                        }
                        ExtractedOp::Write { key, policy, ops } => {
                            let (core_ops, _) = convert_ops_with_ctx_to_core(ops, false)?;
                            batch_ops.push(
                                BatchOperation::write(policy, key.clone(), core_ops)
                            );
                        }
                        ExtractedOp::Delete { key, policy } => {
                            batch_ops.push(
                                BatchOperation::delete(policy, key.clone())
                            );
                        }
                        ExtractedOp::Udf { key, policy, udf_name, function_name, args } => {
                            batch_ops.push(
                                BatchOperation::udf(policy, key.clone(), udf_name, function_name, args.clone().unwrap_or_default())
                            );
                        }
                    }
                }
                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                Ok(batch_ops)
            })?;
            Ok(raw.into_iter().map(|op| BatchRecord { _as: take_batch_record(op) }).collect())
        }

        /// Synchronously execute a streaming batch.
        ///
        /// Same shape as :meth:`batch_blocking` but returns a
        /// :class:`BatchRecordStream` whose items arrive in completion order
        /// (the node that responds first yields first). Each item is a
        /// `(idx, BatchRecord)` tuple. Cluster-level errors raise here;
        /// per-key errors land on each `BatchRecord`'s `result_code`.
        #[pyo3(signature = (ops, *, batch_policy=None))]
        pub fn batch_stream_blocking(
            &self,
            ops: Vec<Py<PyAny>>,
            batch_policy: Option<&BatchPolicy>,
            py: Python<'_>,
        ) -> PyResult<BatchRecordStream> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let extracted = extract_batch_ops_py(py, &ops)?;

            let batch_ops = build_batch_operations(&extracted)?;
            let (driver, stream) = batch_foreach_bridge(client, batch_policy, batch_ops);
            // The global runtime is the one `__next__` drives, so the driver
            // makes progress exactly while the consumer is pulling.
            pyo3_async_runtimes::tokio::get_runtime().spawn(driver);

            Ok(BatchRecordStream {
                inner: Arc::new(Mutex::new(Some(stream))),
                bridge: None,
                closed: Arc::new(AtomicBool::new(false)),
            })
        }

        /// Write record bin(s).
        ///
        /// When `policy_sc` is provided, PNC resolves the key's namespace
        /// mode at op time and picks AP vs SC. The `txn` override is applied
        /// after the mode pick on a cloned policy so caller-cached policies
        /// stay untouched across concurrent ops.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (key, bins, *, policy=None, policy_sc=None, txn=None))]
        pub fn put<'a>(
            &self,
            key: &Key,
            bins: &Bound<'a, PyDict>,
            policy: Option<WritePolicy>,
            policy_sc: Option<WritePolicy>,
            txn: Option<Txn>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let key_as = key._as.clone();
            let client = self._as.clone();

            let bin_vec = bins_from_dict(bins)?;

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let mut policy = match base_sc {
                    Some(sc) => {
                        let is_sc = client.is_strong_consistency(key_as.namespace()).unwrap_or(false);
                        if is_sc { sc } else { base_ap }
                    }
                    None => base_ap,
                };
                if let Some(t) = txn {
                    policy.base_policy.txn = Some(t._as);
                }
                client
                    .put(&policy, &key_as, &bin_vec)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Read record for the specified key.
        ///
        /// When `policy_sc` is provided, PNC resolves the namespace mode at
        /// op time and picks AP vs SC. `filter_expression` / `txn` are
        /// applied after the mode pick on a cloned policy.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (
            key,
            bins=None,
            *,
            policy=None,
            policy_sc=None,
            filter_expression=None,
            txn=None,
        ))]
        pub fn get<'a>(
            &self,
            key: &Key,
            bins: Option<Vec<String>>,
            policy: Option<ReadPolicy>,
            policy_sc: Option<ReadPolicy>,
            filter_expression: Option<FilterExpression>,
            txn: Option<Txn>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let key_as = key._as.clone();
            let client = self._as.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let mut policy = match base_sc {
                    Some(sc) => {
                        let is_sc = client.is_strong_consistency(key_as.namespace()).unwrap_or(false);
                        if is_sc { sc } else { base_ap }
                    }
                    None => base_ap,
                };
                apply_read_overrides(&mut policy, filter_expression, txn);
                let res = client
                    .get(&policy, &key_as, bins_flag(bins))
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;


                Ok(Record { _as: res, cached_bins: None, cached_results: None })
            })
        }

        /// Submit a window of independent single-record reads in one call.
        ///
        /// One crossing spawns all reads and one completion delivers all
        /// results, so per-op submission and wakeup overhead is amortized
        /// across the window. These stay independent wire ops — this is
        /// client-side fusion, not a server batch request. Resolves to
        /// ``(slots, failure_count)``. Slots are positional: each is either
        /// a :class:`Record` or the exception instance for that key (check
        /// with ``isinstance(slot, Exception)``), so a missing record never
        /// fails its window-mates. ``failure_count`` is the number of
        /// exception slots, so a failure-free window is recognizable
        /// without scanning it.
        ///
        /// When `policy_sc` is provided, the namespace mode is resolved at
        /// op time per key and AP vs SC is picked, mirroring `get`.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[tuple[list[typing.Any], int]]", imports=("typing")))]
        #[pyo3(name = "_submit_many_read", signature = (
            keys,
            bins=None,
            *,
            policy=None,
            policy_sc=None,
        ))]
        pub fn submit_many_read<'a>(
            &self,
            keys: Vec<PyRef<Key>>,
            bins: Option<Vec<String>>,
            policy: Option<ReadPolicy>,
            policy_sc: Option<ReadPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let keys_as: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let client = self._as.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let slots = keys_as.into_iter().map(|key_as| {
                    let client = client.clone();
                    let base_ap = base_ap.clone();
                    let base_sc = base_sc.clone();
                    let bins = bins.clone();
                    async move {
                        let policy = match base_sc {
                            Some(sc) => {
                                let is_sc = client
                                    
                                    .is_strong_consistency(key_as.namespace())
                                    .unwrap_or(false);
                                if is_sc { sc } else { base_ap }
                            }
                            None => base_ap,
                        };
                        SlotOutcome(
                            client
                                .get(&policy, &key_as, bins_flag(bins))
                                .await
                                .map(|res| Some(Record {
                                    _as: res,
                                    cached_bins: None,
                                    cached_results: None,
                                }))
                                .map_err(|e| PyErr::from(RustClientError(e))),
                        )
                    }
                });
                Ok(SlotWindow(futures::future::join_all(slots).await))
            })
        }

        /// Per-op-delivery coalesced read: one crossing submits N independent
        /// reads and resolves each caller's OWN pre-created future the moment
        /// that key completes, via the bridge's drainer.
        ///
        /// Unlike :meth:`_submit_many_read`'s single batched future (which
        /// resolves only when the slowest key returns), no op waits on its
        /// window-mates — a task awaiting a fast key resumes immediately and
        /// can issue its next op, so there is no intra-batch head-of-line. This
        /// is the backend for PSDK's transparent same-tick read coalescer.
        ///
        /// Fire-and-forget: returns ``None``. ``futures[i]`` receives key
        /// ``keys[i]``'s :class:`Record`, or its exception (byte-identical to a
        /// direct :meth:`get`, including ``KEY_NOT_FOUND`` raising). `bins` is a
        /// shared projection; `policy_sc`, when set, picks AP vs SC per key.
        #[pyo3(name = "_submit_coalesced_read", signature = (
            keys,
            futures,
            bins=None,
            *,
            policy=None,
            policy_sc=None,
        ))]
        pub fn submit_coalesced_read(
            &self,
            keys: Vec<PyRef<Key>>,
            futures: Vec<Py<PyAny>>,
            bins: Option<Vec<String>>,
            policy: Option<ReadPolicy>,
            policy_sc: Option<ReadPolicy>,
        ) -> PyResult<()> {
            if keys.len() != futures.len() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "keys and futures must have the same length",
                ));
            }
            let bridge = self.require_bridge()?;
            let inner = bridge.inner_arc();
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let client = self._as.clone();
            let pairs: Vec<(aerospike_core::Key, Py<PyAny>)> = keys
                .iter()
                .map(|k| k._as.clone())
                .zip(futures)
                .collect();

            // One spawn drives all N ops concurrently; each delivers its own
            // future as it finishes (FuturesUnordered yields in completion
            // order), so the FFI crossing and Tokio spawn are amortized while
            // delivery stays per-op.
            bridge.spawn(async move {
                use futures::stream::{FuturesUnordered, StreamExt};
                let mut ops = FuturesUnordered::new();
                for (key_as, fut) in pairs {
                    let client = client.clone();
                    let base_ap = base_ap.clone();
                    let base_sc = base_sc.clone();
                    let bins = bins.clone();
                    ops.push(async move {
                        let policy = match base_sc {
                            Some(sc) => {
                                let is_sc = client
                                    
                                    .is_strong_consistency(key_as.namespace())
                                    .unwrap_or(false);
                                if is_sc { sc } else { base_ap }
                            }
                            None => base_ap,
                        };
                        let result = client
                            .get(&policy, &key_as, bins_flag(bins))
                            .await
                            .map(|res| Record {
                                _as: res,
                                cached_bins: None,
                                cached_results: None,
                            })
                            .map_err(|e| PyErr::from(RustClientError(e)));
                        (fut, result)
                    });
                }
                while let Some((fut, result)) = ops.next().await {
                    completion::deliver_one(&inner, fut, result);
                }
            });
            Ok(())
        }

        /// Submit a window of independent single-record writes in one call.
        ///
        /// Write-side counterpart of :meth:`_submit_many_read`: one crossing
        /// spawns all writes and one completion delivers all results. The
        /// bin payload is converted once and shared across the window (the
        /// common benchmark/app shape writes the same record spec per key).
        /// Resolves to ``(slots, failure_count)``: each slot is ``None`` on
        /// success or the exception instance for that key, and
        /// ``failure_count`` is the number of exception slots.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[tuple[list[typing.Any], int]]", imports=("typing")))]
        #[pyo3(name = "_submit_many_write", signature = (
            keys,
            bins,
            *,
            policy=None,
            policy_sc=None,
        ))]
        pub fn submit_many_write<'a>(
            &self,
            keys: Vec<PyRef<Key>>,
            bins: &Bound<'a, PyDict>,
            policy: Option<WritePolicy>,
            policy_sc: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let keys_as: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let client = self._as.clone();

            let bin_vec = std::sync::Arc::new(bins_from_dict(bins)?);

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let slots = keys_as.into_iter().map(|key_as| {
                    let client = client.clone();
                    let base_ap = base_ap.clone();
                    let base_sc = base_sc.clone();
                    let bin_vec = bin_vec.clone();
                    async move {
                        let policy = match base_sc {
                            Some(sc) => {
                                let is_sc = client
                                    
                                    .is_strong_consistency(key_as.namespace())
                                    .unwrap_or(false);
                                if is_sc { sc } else { base_ap }
                            }
                            None => base_ap,
                        };
                        SlotOutcome(
                            client
                                .put(&policy, &key_as, &bin_vec)
                                .await
                                .map(|_| None)
                                .map_err(|e| PyErr::from(RustClientError(e))),
                        )
                    }
                });
                Ok(SlotWindow(futures::future::join_all(slots).await))
            })
        }

        /// Per-op-delivery coalesced write: one crossing submits N independent
        /// writes, each carrying its OWN bin payload, and resolves each
        /// caller's pre-created future the moment that key completes.
        ///
        /// Write-side counterpart of :meth:`_submit_coalesced_read`. The
        /// distinction from :meth:`_submit_many_write` is per-key payloads:
        /// that method broadcasts ONE ``bins`` dict to every key and resolves a
        /// single batched future, so it cannot carry writes that differ per
        /// key; here ``bins_list[i]`` is the payload for ``keys[i]``. That is
        /// what lets a caller-side buffer of unrelated writes fuse into one
        /// crossing. As with the read side, no op waits on its window-mates —
        /// there is no intra-batch head-of-line.
        ///
        /// Fire-and-forget: returns ``None``. ``futures[i]`` receives ``None``
        /// on success, or key ``keys[i]``'s exception (byte-identical to a
        /// direct :meth:`put`). `policy_sc`, when set, picks AP vs SC per key.
        #[pyo3(name = "_submit_coalesced_write", signature = (
            keys,
            futures,
            bins_list,
            *,
            policy=None,
            policy_sc=None,
        ))]
        pub fn submit_coalesced_write(
            &self,
            keys: Vec<PyRef<Key>>,
            futures: Vec<Py<PyAny>>,
            bins_list: Vec<Py<PyAny>>,
            policy: Option<WritePolicy>,
            policy_sc: Option<WritePolicy>,
            py: Python<'_>,
        ) -> PyResult<()> {
            if keys.len() != futures.len() || keys.len() != bins_list.len() {
                return Err(PyValueError::new_err(
                    "keys, futures, and bins_list must have the same length",
                ));
            }
            let bridge = self.require_bridge()?;
            let inner = bridge.inner_arc();
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let client = self._as.clone();

            let ops: Vec<(aerospike_core::Key, Py<PyAny>, Vec<aerospike_core::Bin>)> = keys
                .iter()
                .map(|k| k._as.clone())
                .zip(futures)
                .zip(bins_from_dict_list(py, &bins_list)?)
                .map(|((key_as, fut), bin_vec)| (key_as, fut, bin_vec))
                .collect();

            // One spawn drives all N ops concurrently; each delivers its own
            // future as it finishes (FuturesUnordered yields in completion
            // order), so the FFI crossing and Tokio spawn are amortized while
            // delivery stays per-op.
            bridge.spawn(async move {
                use futures::stream::{FuturesUnordered, StreamExt};
                let mut pending = FuturesUnordered::new();
                for (key_as, fut, bin_vec) in ops {
                    let client = client.clone();
                    let base_ap = base_ap.clone();
                    let base_sc = base_sc.clone();
                    pending.push(async move {
                        let policy = match base_sc {
                            Some(sc) => {
                                let is_sc = client
                                    
                                    .is_strong_consistency(key_as.namespace())
                                    .unwrap_or(false);
                                if is_sc { sc } else { base_ap }
                            }
                            None => base_ap,
                        };
                        // `None` so the future resolves to Python `None`, matching
                        // a direct `put`; a Rust unit would cross as `()`.
                        let result = client
                            .put(&policy, &key_as, &bin_vec)
                            .await
                            .map(|_| Option::<Record>::None)
                            .map_err(|e| PyErr::from(RustClientError(e)));
                        (fut, result)
                    });
                }
                while let Some((fut, result)) = pending.next().await {
                    completion::deliver_one(&inner, fut, result);
                }
            });
            Ok(())
        }

        /// Execute multiple operations atomically on a single record.
        ///
        /// When `policy_sc` is provided, PNC resolves the namespace mode at
        /// op time and picks AP vs SC. The full override set is applied
        /// after the mode pick on a cloned policy.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[Record]", imports=("typing", "aerospike_native")))]
        #[pyo3(signature = (
            key,
            operations,
            *,
            policy=None,
            policy_sc=None,
            record_exists_action=None,
            expiration=None,
            generation=None,
            durable_delete=None,
            filter_expression=None,
            txn=None,
        ))]
        pub fn operate<'a>(
            &self,
            key: &Key,
            operations: Vec<Py<PyAny>>,
            policy: Option<WritePolicy>,
            policy_sc: Option<WritePolicy>,
            record_exists_action: Option<RecordExistsAction>,
            expiration: Option<Expiration>,
            generation: Option<u32>,
            durable_delete: Option<bool>,
            filter_expression: Option<FilterExpression>,
            txn: Option<Txn>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let base_ap = policy.map(|p| p._as.clone()).unwrap_or_default();
            let base_sc = policy_sc.map(|p| p._as.clone());
            let key_as = key._as.clone();
            let client = self._as.clone();

            let rust_ops = extract_py_ops_with_ctx(py, &operations)?;

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let mut policy = match base_sc {
                    Some(sc) => {
                        let is_sc = client.is_strong_consistency(key_as.namespace()).unwrap_or(false);
                        if is_sc { sc } else { base_ap }
                    }
                    None => base_ap,
                };
                apply_write_overrides(
                    &mut policy,
                    record_exists_action,
                    expiration,
                    generation,
                    durable_delete,
                    filter_expression,
                    txn,
                );
                let (core_ops, _) = convert_ops_with_ctx_to_core(&rust_ops, false)?;
                let res = client
                    .operate(&policy, &key_as, &core_ops)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(Record { _as: res, cached_bins: None, cached_results: None })
            })
        }

        /// Add integer bin values to existing record bin values. The policy specifies the transaction
        /// timeout, record expiration and how the transaction is handled when the record already
        /// exists. This call only works for integer values.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (key, bins, *, policy=None))]
        pub fn add<'a>(
            &self,
            key: &Key,
            bins: HashMap<String, PythonValue>,
            policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();

            let bins: Vec<aerospike_core::Bin> = bins
                .into_iter()
                .map(|(name, val)| aerospike_core::Bin::new(name, val.into()))
                .collect();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .add(&policy, &key, &bins)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Append bin string values to existing record bin values. The policy specifies the
        /// transaction timeout, record expiration and how the transaction is handled when the record
        /// already exists. This call only works for string values.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (key, bins, *, policy=None))]
        pub fn append<'a>(
            &self,
            key: &Key,
            bins: HashMap<String, PythonValue>,
            policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();

            let bins: Vec<aerospike_core::Bin> = bins
                .into_iter()
                .map(|(name, val)| aerospike_core::Bin::new(name, val.into()))
                .collect();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .append(&policy, &key, &bins)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Prepend bin string values to existing record bin values. The policy specifies the
        /// transaction timeout, record expiration and how the transaction is handled when the record
        /// already exists. This call only works for string values.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (key, bins, *, policy=None))]
        pub fn prepend<'a>(
            &self,
            key: &Key,
            bins: HashMap<String, PythonValue>,
            policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();

            let bins: Vec<aerospike_core::Bin> = bins
                .into_iter()
                .map(|(name, val)| aerospike_core::Bin::new(name, val.into()))
                .collect();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .prepend(&policy, &key, &bins)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Delete record for specified key. The policy specifies the transaction timeout.
        /// The call returns `true` if the record existed on the server before deletion.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (key, *, policy=None))]
        pub fn delete<'a>(
            &self,
            key: &Key,
            policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let res = client
                    .delete(&policy, &key)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(res)
            })
        }

        /// Reset record's time to expiration using the policy's expiration. Fail if the record does
        /// not exist.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (key, *, policy=None))]
        pub fn touch<'a>(
            &self,
            key: &Key,
            policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .touch(&policy, &key)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Read multiple records for specified keys in one batch call.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.List[BatchRecord]]", imports=("typing")))]
        #[pyo3(signature = (keys, bins=None, *, batch_policy=None, read_policy=None))]
        pub fn batch_read<'a>(
            &self,
            keys: Vec<PyRef<Key>>,
            bins: Option<Vec<String>>,
            batch_policy: Option<&BatchPolicy>,
            read_policy: Option<&BatchReadPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let read_policy = read_policy.map(|p| p._as.clone()).unwrap_or_default();

            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();
            let client = self._as.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                use aerospike_core::BatchOperation;

                let bf = bins_flag(bins);
                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    batch_ops.push(BatchOperation::read(&read_policy, key, bf.clone()));
                }

                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                let results = batch_ops;

                Ok(results
                    .into_iter()
                    .map(|op| BatchRecord { _as: take_batch_record(op) })
                    .collect::<Vec<BatchRecord>>())
            })
        }

        /// Write multiple records for specified keys in one batch call.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.List[BatchRecord]]", imports=("typing")))]
        #[pyo3(signature = (keys, bins_list, *, batch_policy=None, write_policy=None))]
        pub fn batch_write<'a>(
            &self,
            keys: Vec<PyRef<Key>>,
            bins_list: Vec<Py<PyAny>>,
            batch_policy: Option<&BatchPolicy>,
            write_policy: Option<&BatchWritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            if keys.len() != bins_list.len() {
                return Err(PyValueError::new_err("keys and bins_list must have the same length"));
            }

            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let write_policy = write_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();

            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();

            let bins_vecs = bins_from_dict_list(py, &bins_list)?;

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                use aerospike_core::BatchOperation;
                use aerospike_core::operations;

                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for (key, bins) in rust_keys.into_iter().zip(bins_vecs.iter()) {
                    let ops: Vec<aerospike_core::operations::Operation> = bins
                        .iter()
                        .map(operations::put)
                        .collect();
                    batch_ops.push(BatchOperation::write(&write_policy, key, ops));
                }

                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                let results = batch_ops;

                Ok(results
                    .into_iter()
                    .map(|op| BatchRecord { _as: take_batch_record(op) })
                    .collect::<Vec<BatchRecord>>())
            })
        }

        /// Perform read/write operations on multiple keys in one batch call.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.List[BatchRecord]]", imports=("typing")))]
        #[pyo3(signature = (keys, operations_list, *, batch_policy=None, write_policy=None))]
        pub fn batch_operate<'a>(
            &self,
            keys: Vec<PyRef<Key>>,
            operations_list: Vec<Vec<Py<PyAny>>>,
            batch_policy: Option<&BatchPolicy>,
            write_policy: Option<&BatchWritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            if keys.len() != operations_list.len() {
                return Err(PyValueError::new_err("keys and operations_list must have the same length"));
            }

            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let write_policy = write_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();

            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();

            let mut rust_ops_list = Vec::with_capacity(operations_list.len());
            for operations in operations_list {
                rust_ops_list.push(extract_py_ops_with_ctx(py, &operations)?);
            }

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                use aerospike_core::BatchOperation;

                let read_policy = aerospike_core::BatchReadPolicy::default();

                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for (key, ops) in rust_keys.into_iter().zip(rust_ops_list) {
                    let (core_ops, has_write_op) = convert_ops_with_ctx_to_core(&ops, true)?;
                    let batch_op = if has_write_op {
                        BatchOperation::write(&write_policy, key, core_ops)
                    } else {
                        BatchOperation::read_ops(&read_policy, key, core_ops)
                    };
                    batch_ops.push(batch_op);
                }

                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                let results = batch_ops;

                Ok(results
                    .into_iter()
                    .map(|op| BatchRecord { _as: take_batch_record(op) })
                    .collect::<Vec<BatchRecord>>())
            })
        }

        /// Delete multiple records for specified keys in one batch call.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.List[BatchRecord]]", imports=("typing")))]
        #[pyo3(signature = (keys, *, batch_policy=None, delete_policy=None))]
        pub fn batch_delete<'a>(
            &self,
            keys: Vec<PyRef<Key>>,
            batch_policy: Option<&BatchPolicy>,
            delete_policy: Option<&BatchDeletePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let delete_policy = delete_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();

            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                use aerospike_core::BatchOperation;

                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    batch_ops.push(BatchOperation::delete(&delete_policy, key));
                }

                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                let results = batch_ops;

                Ok(results
                    .into_iter()
                    .map(|op| BatchRecord { _as: take_batch_record(op) })
                    .collect::<Vec<BatchRecord>>())
            })
        }

        /// Check if multiple record keys exist in one batch call.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.List[builtins.bool]]", imports=("typing", "builtins")))]
        #[pyo3(signature = (keys, *, batch_policy=None, read_policy=None))]
        pub fn batch_exists<'a>(
            &self,
            keys: Vec<PyRef<Key>>,
            batch_policy: Option<&BatchPolicy>,
            read_policy: Option<&BatchReadPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let read_policy = read_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();

            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                use aerospike_core::BatchOperation;
                use aerospike_core::Bins;

                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    batch_ops.push(BatchOperation::read(&read_policy, key, Bins::None));
                }

                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                let results = batch_ops;

                Ok(results
                    .iter()
                    .map(|op| op.record().is_some())
                    .collect::<Vec<bool>>())
            })
        }

        /// Read multiple record headers (metadata only, no bin data) for specified keys in one batch call.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.List[typing.Optional[Record]]]", imports=("typing")))]
        #[pyo3(signature = (keys, *, batch_policy=None, read_policy=None))]
        pub fn batch_get_header<'a>(
            &self,
            keys: Vec<PyRef<Key>>,
            batch_policy: Option<&BatchPolicy>,
            read_policy: Option<&BatchReadPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let read_policy = read_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();

            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                use aerospike_core::BatchOperation;
                use aerospike_core::Bins;

                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    batch_ops.push(BatchOperation::read(&read_policy, key, Bins::None));
                }

                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                let results = batch_ops;

                Ok(results
                    .into_iter()
                    .map(|op| {
                        take_batch_record(op)
                            .record
                            .map(|r| Record { _as: r, cached_bins: None, cached_results: None })
                    })
                    .collect::<Vec<Option<Record>>>())
            })
        }

        /// Apply UDF operations on multiple keys in one batch call.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.List[BatchRecord]]", imports=("typing")))]
        #[pyo3(signature = (keys, udf_name, function_name, args, *, batch_policy=None, udf_policy=None))]
        pub fn batch_apply<'a>(
            &self,
            keys: Vec<PyRef<Key>>,
            udf_name: String,
            function_name: String,
            args: Option<Vec<PythonValue>>,
            batch_policy: Option<&BatchPolicy>,
            udf_policy: Option<&BatchUDFPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let udf_policy = udf_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();

            let rust_keys: Vec<aerospike_core::Key> =
                keys.iter().map(|k| k._as.clone()).collect();

            let rust_args = args.map(|args| {
                args.into_iter()
                    .map(|v| v.into())
                    .collect::<Vec<aerospike_core::Value>>()
            });

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                use aerospike_core::BatchOperation;

                let mut batch_ops = Vec::with_capacity(rust_keys.len());
                for key in rust_keys {
                    let rust_args_owned = rust_args.clone().unwrap_or_default();
                    batch_ops.push(BatchOperation::udf(&udf_policy, key, &udf_name, &function_name, rust_args_owned));
                }

                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                let results = batch_ops;

                Ok(results
                    .into_iter()
                    .map(|op| BatchRecord { _as: take_batch_record(op) })
                    .collect::<Vec<BatchRecord>>())
            })
        }

        /// Execute a mixed batch of read, write, and delete operations in a single server call.
        ///
        /// Each operation is specified via :class:`BatchReadOp`, :class:`BatchWriteOp`,
        /// :class:`BatchDeleteOp`, or :class:`BatchUDFOp`, each carrying its own key
        /// and per-record policy.
        ///
        /// Args:
        ///     batch_policy: Optional :class:`BatchPolicy` for the entire batch.
        ///     ops: List of :class:`BatchReadOp`, :class:`BatchWriteOp`,
        ///          :class:`BatchDeleteOp`, and/or :class:`BatchUDFOp` objects.
        ///
        /// Returns:
        ///     A list of :class:`BatchRecord` results in the same order as the input ops.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Sequence[BatchRecord]]", imports=("typing")))]
        #[pyo3(signature = (ops, *, batch_policy=None))]
        pub fn batch<'a>(
            &self,
            ops: Vec<Py<PyAny>>,
            batch_policy: Option<&BatchPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();

            #[derive(Clone)]
            enum ExtractedOp {
                Read {
                    key: aerospike_core::Key,
                    policy: aerospike_core::BatchReadPolicy,
                    bins: Option<Vec<String>>,
                    ops: Vec<OpWithCtx>,
                },
                Write {
                    key: aerospike_core::Key,
                    policy: aerospike_core::BatchWritePolicy,
                    ops: Vec<OpWithCtx>,
                },
                Delete {
                    key: aerospike_core::Key,
                    policy: aerospike_core::BatchDeletePolicy,
                },
                Udf {
                    key: aerospike_core::Key,
                    policy: aerospike_core::BatchUdfPolicy,
                    udf_name: String,
                    function_name: String,
                    args: Option<Vec<aerospike_core::Value>>,
                },
            }

            let mut extracted: Vec<ExtractedOp> = Vec::with_capacity(ops.len());
            for op_obj in &ops {
                if let Ok(read_op) = op_obj.extract::<PyRef<BatchReadOp>>(py) {
                    extracted.push(ExtractedOp::Read {
                        key: read_op.key.clone(),
                        policy: read_op.policy.clone(),
                        bins: read_op.bins.clone(),
                        ops: read_op.ops.clone(),
                    });
                } else if let Ok(write_op) = op_obj.extract::<PyRef<BatchWriteOp>>(py) {
                    extracted.push(ExtractedOp::Write {
                        key: write_op.key.clone(),
                        policy: write_op.policy.clone(),
                        ops: write_op.ops.clone(),
                    });
                } else if let Ok(delete_op) = op_obj.extract::<PyRef<BatchDeleteOp>>(py) {
                    extracted.push(ExtractedOp::Delete {
                        key: delete_op.key.clone(),
                        policy: delete_op.policy.clone(),
                    });
                } else if let Ok(udf_op) = op_obj.extract::<PyRef<BatchUDFOp>>(py) {
                    extracted.push(ExtractedOp::Udf {
                        key: udf_op.key.clone(),
                        policy: udf_op.policy.clone(),
                        udf_name: udf_op.udf_name.clone(),
                        function_name: udf_op.function_name.clone(),
                        args: udf_op.args.clone(),
                    });
                } else {
                    return Err(PyTypeError::new_err(
                        "Each op must be a BatchReadOp, BatchWriteOp, BatchDeleteOp, or BatchUDFOp"
                    ));
                }
            }

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                use aerospike_core::BatchOperation;

                let mut batch_ops = Vec::with_capacity(extracted.len());
                for ext in &extracted {
                    match ext {
                        ExtractedOp::Read { key, policy, bins, ops } if ops.is_empty() => {
                            batch_ops.push(
                                BatchOperation::read(policy, key.clone(), bins_flag(bins.clone()))
                            );
                        }
                        ExtractedOp::Read { key, policy, bins: _, ops } => {
                            let (core_ops, _has_write) = convert_ops_with_ctx_to_core(ops, false)?;
                            batch_ops.push(
                                BatchOperation::read_ops(policy, key.clone(), core_ops)
                            );
                        }
                        ExtractedOp::Write { key, policy, ops } => {
                            let (core_ops, _) = convert_ops_with_ctx_to_core(ops, false)?;
                            batch_ops.push(
                                BatchOperation::write(policy, key.clone(), core_ops)
                            );
                        }
                        ExtractedOp::Delete { key, policy } => {
                            batch_ops.push(
                                BatchOperation::delete(policy, key.clone())
                            );
                        }
                        ExtractedOp::Udf { key, policy, udf_name, function_name, args } => {
                            batch_ops.push(
                                BatchOperation::udf(policy, key.clone(), udf_name, function_name, args.clone().unwrap_or_default())
                            );
                        }
                    }
                }

                if let Err(e) = client.batch(&batch_policy, &mut batch_ops).await {
                    return Err(batch_failed_with_rows(e, batch_ops));
                }
                let results = batch_ops;

                Ok(results
                    .into_iter()
                    .map(|op| BatchRecord { _as: take_batch_record(op) })
                    .collect::<Vec<BatchRecord>>())
            })
        }

        /// Asynchronously execute a streaming batch.
        ///
        /// Returns an awaitable resolving to a :class:`BatchRecordStream`.
        /// Items arrive in completion order (the node that responds first
        /// yields first). Each item is a `(idx, BatchRecord)` tuple where
        /// ``idx`` is the position of the originating op in ``ops``.
        /// Per-key errors land on each `BatchRecord`'s ``result_code``;
        /// cluster-level errors raise from the resolving awaitable or
        /// from ``__anext__``.
        #[gen_stub(override_return_type(
            type_repr="typing.Awaitable[BatchRecordStream]",
            imports=("typing", "aerospike_native"),
        ))]
        #[pyo3(signature = (ops, *, batch_policy=None))]
        pub fn batch_stream<'a>(
            &self,
            ops: Vec<Py<PyAny>>,
            batch_policy: Option<&BatchPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let batch_policy = batch_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let extracted = extract_batch_ops_py(py, &ops)?;
            // Clone the bridge before the spawn so the resulting stream can
            // route each per-yield future through the same loop + per-Client
            // runtime as the Client itself.
            let bridge = self.require_bridge()?;
            let stream_bridge = bridge.clone();

            completion::batched_future_into_py(bridge, py, async move {
                let batch_ops = build_batch_operations(&extracted)?;
                let (driver, stream) = batch_foreach_bridge(client, batch_policy, batch_ops);
                // Same reactor as the ops it drives (per-Client runtime under
                // AsyncPool isolation, global otherwise).
                stream_bridge.spawn(driver);
                Ok(BatchRecordStream {
                    inner: Arc::new(Mutex::new(Some(stream))),
                    bridge: Some(stream_bridge),
                    closed: Arc::new(AtomicBool::new(false)),
                })
            })
        }

        /// Execute a UDF (User Defined Function) on a single record.
        ///
        /// Args:
        ///     policy: WritePolicy for the operation.
        ///     key: The key of the record to execute the UDF on.
        ///     server_path: Server path to the UDF module (e.g., "example.lua").
        ///     function_name: Name of the function to execute within the UDF module.
        ///     args: Optional list of arguments to pass to the UDF function.
        ///
        /// Returns:
        ///     Optional Value containing the UDF result, or None if the UDF returns no value.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Optional[typing.Any]]", imports=("typing")))]
        #[pyo3(signature = (key, server_path, function_name, args=None, *, policy=None))]
        pub fn execute_udf<'a>(
            &self,
            key: &Key,
            server_path: String,
            function_name: String,
            args: Option<Vec<PythonValue>>,
            policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();

            // Convert args before moving into async block
            let rust_args = args.map(|args| {
                args.into_iter()
                    .map(|v| v.into())
                    .collect::<Vec<aerospike_core::Value>>()
            });

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let rust_args_ref = rust_args.as_deref().unwrap_or(&[]);
                let result = client
                    .execute_udf(&policy, &key, &server_path, &function_name, rust_args_ref)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                // Return `Option<PythonValue>` directly — pyo3-async-runtimes
                // performs a single `Python::attach` at future-delivery time and
                // invokes `IntoPyObject` then.  Avoids the redundant
                // `Python::attach` inside the Tokio worker that this method
                // previously did purely to wrap the result.
                Ok(result.map(PythonValue::from))
            })
        }

        /// Execute a query/scan and apply write operations to matching records (background job).
        /// Returns an ExecuteTask to poll for completion. Accepts any write operation: scalar
        /// (put, add, append, prepend, delete, touch), expression, list, map, bit, HLL, string
        /// and path operations, including nested CDT context. The server rejects read
        /// operations in a background job.
        ///
        /// Args:
        ///     write_policy: WritePolicy for the background operation.
        ///     statement: Statement (namespace, set, optional filters).
        ///     operations: Write operations to apply to each matching record (e.g.
        ///         Operation.put, MapOperation.remove_by_value_range, HllOperation.add).
        ///
        /// Returns:
        ///     ExecuteTask to monitor completion (query_status, wait_till_complete).
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[ExecuteTask]", imports=("typing")))]
        #[pyo3(signature = (statement, operations, *, write_policy=None))]
        pub fn query_operate<'a>(
            &self,
            statement: &Statement,
            operations: Vec<Py<PyAny>>,
            write_policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = write_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let mut core_statement = statement._as.clone();

            let rust_ops = extract_py_ops_with_ctx(py, &operations)?;
            let (core_ops, _) = convert_ops_with_ctx_to_core(&rust_ops, false)?;
            core_statement.set_operations(core_ops);

            let bridge = self.require_bridge()?;
            let task_bridge = bridge.clone();
            completion::batched_future_into_py(bridge, py, async move {
                let task = client
                    .query_operate(&policy, core_statement)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(ExecuteTask { _as: task, bridge: Some(task_bridge) })
            })
        }

        /// Apply a UDF to records matching the statement filter (background job).
        /// Returns an ExecuteTask to poll for completion. Records are not returned.
        /// If the statement has no filter, the UDF is applied to all records in the namespace/set.
        ///
        /// Args:
        ///     write_policy: WritePolicy for the background operation.
        ///     statement: Statement (namespace, set, optional filters).
        ///     package_name: Server-side UDF package name.
        ///     function_name: UDF function to invoke.
        ///     args: Optional arguments to pass to the UDF.
        ///
        /// Returns:
        ///     ExecuteTask to monitor completion (query_status, wait_till_complete).
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[ExecuteTask]", imports=("typing")))]
        #[pyo3(signature = (statement, package_name, function_name, args=None, *, write_policy=None))]
        pub fn query_execute_udf<'a>(
            &self,
            statement: &Statement,
            package_name: String,
            function_name: String,
            args: Option<Vec<PythonValue>>,
            write_policy: Option<WritePolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = write_policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let mut core_statement = statement._as.clone();
            let rust_args = args.map(|a| a.into_iter().map(|v| v.into()).collect::<Vec<aerospike_core::Value>>());
            core_statement.set_aggregate_function(&package_name, &function_name, rust_args.as_deref().unwrap_or(&[]));

            let bridge = self.require_bridge()?;
            let task_bridge = bridge.clone();
            completion::batched_future_into_py(bridge, py, async move {
                let args_ref = rust_args.as_deref().unwrap_or(&[]);
                let task = client
                    .query_execute_udf(&policy, core_statement, &package_name, &function_name, args_ref)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(ExecuteTask { _as: task, bridge: Some(task_bridge) })
            })
        }

        /// Register a UDF (User Defined Function) module on the server from bytes.
        ///
        /// Args:
        ///     policy: AdminPolicy for the operation.
        ///     udf_body: The UDF module content as bytes.
        ///     server_path: Server path where the UDF will be stored (e.g., "example.lua").
        ///     language: UDF language (UDFLang.LUA).
        ///
        /// Returns:
        ///     RegisterTask that can be used to wait for registration completion.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[RegisterTask]", imports=("typing")))]
        #[pyo3(signature = (udf_body, server_path, language, *, policy=None))]
        pub fn register_udf<'a>(
            &self,
            udf_body: Vec<u8>,
            server_path: String,
            language: UDFLang,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();
            let lang: aerospike_core::UdfLang = language.into();

            let bridge = self.require_bridge()?;
            let task_bridge = bridge.clone();
            completion::batched_future_into_py(bridge, py, async move {
                let task = client
                    .register_udf(&admin_policy, &udf_body, &server_path, lang)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(RegisterTask { _as: task, bridge: Some(task_bridge) })
            })
        }

        /// Register a UDF (User Defined Function) module on the server from a file.
        ///
        /// Args:
        ///     policy: AdminPolicy for the operation.
        ///     client_path: Local file path to the UDF module.
        ///     server_path: Server path where the UDF will be stored (e.g., "example.lua").
        ///     language: UDF language (UDFLang.LUA).
        ///
        /// Returns:
        ///     RegisterTask that can be used to wait for registration completion.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[RegisterTask]", imports=("typing")))]
        #[pyo3(signature = (client_path, server_path, language, *, policy=None))]
        pub fn register_udf_from_file<'a>(
            &self,
            client_path: String,
            server_path: String,
            language: UDFLang,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();
            let lang: aerospike_core::UdfLang = language.into();

            let bridge = self.require_bridge()?;
            let task_bridge = bridge.clone();
            completion::batched_future_into_py(bridge, py, async move {
                let task = client
                    .register_udf_from_file(&admin_policy, &client_path, &server_path, lang)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(RegisterTask { _as: task, bridge: Some(task_bridge) })
            })
        }

        /// Remove a UDF (User Defined Function) module from the server.
        ///
        /// Args:
        ///     policy: AdminPolicy for the operation.
        ///     server_path: Server path to the UDF module to remove (e.g., "example.lua").
        ///
        /// Returns:
        ///     UdfRemoveTask that can be used to wait for removal completion.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[UdfRemoveTask]", imports=("typing")))]
        #[pyo3(signature = (server_path, *, policy=None))]
        pub fn remove_udf<'a>(
            &self,
            server_path: String,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            let bridge = self.require_bridge()?;
            let task_bridge = bridge.clone();
            completion::batched_future_into_py(bridge, py, async move {
                let task = client
                    .remove_udf(&admin_policy, &server_path)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(UdfRemoveTask { _as: task, bridge: Some(task_bridge) })
            })
        }

        /// Determine if a record key exists. The policy can be used to specify timeouts.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (key, *, policy=None))]
        pub fn exists<'a>(
            &self,
            key: &Key,
            policy: Option<ReadPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let res = Self::exists_internal(client, policy, key)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(res)
            })
        }

        /// Determine if a record key exists. Returns (key, meta) where meta=None if record not found.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Tuple[Key, typing.Optional[typing.Any]]]", imports=("typing")))]
        #[pyo3(signature = (key, *, policy=None))]
        pub fn exists_legacy<'a>(
            &self,
            key: &Key,
            policy: Option<ReadPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let key = key._as.clone();
            let client = self._as.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let exists = Self::exists_internal(client.clone(), policy.clone(), key.clone())
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                // If the record exists, fetch metadata via a header-only get
                // (Bins::None).  Otherwise meta is None.
                let meta_record = if exists {
                    let read_policy = aerospike_core::ReadPolicy::default();
                    Some(
                        client
                            .get(&read_policy, &key, aerospike_core::Bins::None)
                            .await
                            .map_err(|e| PyErr::from(RustClientError(e)))?,
                    )
                } else {
                    None
                };

                Ok(PendingExistsLegacy { key, meta_record })
            })
        }

        /// Removes all records in the specified namespace/set efficiently.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (namespace, set_name, before_nanos = None, *, policy = None))]
        pub fn truncate<'a>(
            &self,
            namespace: String,
            set_name: String,
            before_nanos: Option<i64>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            let before_nanos = before_nanos.unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .truncate(&admin_policy, &namespace, &set_name, before_nanos)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Create a secondary index on a bin containing scalar values. Returns an
        /// :class:`IndexTask`; the server builds the index asynchronously, so await
        /// :meth:`IndexTask.wait_till_complete` before querying through it.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[IndexTask]", imports=("typing")))]
        #[pyo3(signature = (namespace, set_name, bin_name, index_name, index_type, cit = None, ctx = None, *, policy = None))]
        pub fn create_index<'a>(
            &self,
            namespace: String,
            set_name: String,
            bin_name: String,
            index_name: String,
            index_type: IndexType,
            cit: Option<CollectionIndexType>,
            ctx: Option<Vec<CTX>>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            let cit = (&cit.unwrap_or(CollectionIndexType::Default)).into();
            let index_type = (&index_type).into();
            let ctx_core = ctx.map(|c| ctx_to_vec(&c));

            let bridge = self.require_bridge()?;
            let task_bridge = bridge.clone();
            completion::batched_future_into_py(bridge, py, async move {
                let task = client
                    .create_index_on_bin(
                        &admin_policy,
                        &namespace,
                        &set_name,
                        &bin_name,
                        &index_name,
                        index_type,
                        cit,
                        ctx_core.as_deref(),
                    )
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(IndexTask { _as: task, bridge: Some(task_bridge) })
            })
        }

        /// Create a set index: a secondary index on record presence per set,
        /// with no bin, type, context or expression. Only the ``sindex-admin``
        /// privilege is needed. Requires server 8.1.2 or later. Returns an
        /// :class:`IndexTask`; await :meth:`IndexTask.wait_till_complete`
        /// before relying on the index.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[IndexTask]", imports=("typing")))]
        #[pyo3(signature = (namespace, set_name, index_name, *, policy = None))]
        pub fn create_set_index<'a>(
            &self,
            namespace: String,
            set_name: String,
            index_name: String,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            let bridge = self.require_bridge()?;
            let task_bridge = bridge.clone();
            completion::batched_future_into_py(bridge, py, async move {
                let task = client
                    .create_set_index(&admin_policy, &namespace, &set_name, &index_name)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(IndexTask { _as: task, bridge: Some(task_bridge) })
            })
        }

        /// Drop a secondary index. Returns a DropIndexTask to track completion.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[DropIndexTask]", imports=("typing")))]
        #[pyo3(signature = (namespace, set_name, index_name, *, policy = None))]
        pub fn drop_index<'a>(
            &self,
            namespace: String,
            set_name: String,
            index_name: String,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            let bridge = self.require_bridge()?;
            let task_bridge = bridge.clone();
            completion::batched_future_into_py(bridge, py, async move {
                let task = client
                    .drop_index(&admin_policy, &namespace, &set_name, &index_name)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(DropIndexTask { _as: task, bridge: Some(task_bridge) })
            })
        }

        /// Create a secondary index using an expression. Returns an IndexTask to wait for completion.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[IndexTask]", imports=("typing")))]
        #[pyo3(signature = (namespace, set_name, index_name, index_type, expression, cit = None, *, policy = None))]
        pub fn create_index_using_expression<'a>(
            &self,
            namespace: String,
            set_name: String,
            index_name: String,
            index_type: IndexType,
            expression: &FilterExpression,
            cit: Option<CollectionIndexType>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy =
                policy.map(|p| p._as).unwrap_or_default();
            let expr = expression._as.clone();
            let cit = (&cit.unwrap_or(CollectionIndexType::Default)).into();
            let index_type = (&index_type).into();

            let bridge = self.require_bridge()?;
            let task_bridge = bridge.clone();
            completion::batched_future_into_py(bridge, py, async move {
                let task = client
                    .create_index_using_expression(
                        &admin_policy,
                        &namespace,
                        &set_name,
                        &index_name,
                        index_type,
                        cit,
                        &expr,
                    )
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(IndexTask { _as: task, bridge: Some(task_bridge) })
            })
        }

        /// Execute a query on all server nodes and return a record iterator. The query executor puts
        /// records on a queue in separate threads. The calling thread concurrently pops records off
        /// the queue through the record iterator.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (statement, partition_filter, *, policy=None))]
        pub fn query<'a>(
            &self,
            statement: &Statement,
            partition_filter: PartitionFilter,
            policy: Option<QueryPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let stmt = statement.clone()._as;
            let bridge = self.require_bridge()?;
            let recordset_bridge = bridge.clone();

            completion::batched_future_into_py(bridge, py, async move {
                let res = client
                    .query(&policy, partition_filter._as, stmt)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(Recordset {
                    _as: res,
                    _stream: Arc::new(Mutex::new(None)),
                    bridge: Some(recordset_bridge),
                })
            })
        }

        /// Run phase 1 (explain) of server-led query selection.
        ///
        /// Callers must verify :meth:`Version.supports_query_selection` before use;
        /// this method sends field ``44`` regardless of server version.
        ///
        /// Args:
        ///     ael: Filter expression in Aerospike Expression Language; sent on
        ///         field ``44`` and compiled server-side during explain.
        ///     index_name_hint: Field ``21`` soft index-name hint. Becomes strict
        ///         when ``HARD_HINT`` is set in ``explain_where_flags`` — explain then
        ///         fails unless that exact index is selected.
        ///     explain_where_flags: Tier-D field ``44`` bits —
        ///         ``QueryWhereFlags.EXPLAIN`` plus optional ``REQUIRE_INDEX`` /
        ///         ``HARD_HINT``. Omit for default explain.
        ///
        /// A plan with :attr:`QueryPlan.selection` ``FILTERED_OUT`` must not be
        /// passed to :meth:`query_with_plan` (raises ``FilteredOut``).
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[QueryPlan]", imports=("typing")))]
        #[pyo3(signature = (namespace, ael, *, set_name=None, index_name_hint=None, explain_where_flags=None, policy=None))]
        pub fn query_explain<'a>(
            &self,
            namespace: String,
            ael: String,
            set_name: Option<String>,
            index_name_hint: Option<String>,
            explain_where_flags: Option<u8>,
            policy: Option<QueryPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let where_flags = explain_where_flags.unwrap_or(aerospike_core::FLAG_EXPLAIN);
            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let plan = client
                    .query_explain(
                        &policy,
                        &namespace,
                        set_name.as_deref(),
                        &ael,
                        index_name_hint.as_deref(),
                        explain_where_flags,
                    )
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                QueryPlan::from_core(plan, where_flags)
            })
        }

        /// Execute a partitioned query using a server query plan from
        /// :meth:`query_explain`.
        ///
        /// The index filter is taken from ``plan``; do not set
        /// :attr:`Statement.filters`. ``statement`` must match ``plan`` namespace
        /// and set; a mismatch raises :exc:`ValueError`.
        ///
        /// Args:
        ///     statement: Bins to return; must match the plan namespace/set.
        ///     partition_filter: Partition scope for the query.
        ///     plan: Plan from :meth:`query_explain`.
        ///     policy: Optional query policy.
        ///
        /// Raises:
        ///     ValueError: If the plan's namespace/set does not match
        ///         ``statement`` (validated client-side before any server round trip),
        ///         or if ``statement`` already has filters.
        ///     FilteredOut: If ``plan.selection`` is ``FILTERED_OUT``.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[Recordset]", imports=("typing")))]
        #[pyo3(signature = (statement, partition_filter, plan, *, policy=None))]
        pub fn query_with_plan<'a>(
            &self,
            statement: &Statement,
            partition_filter: PartitionFilter,
            plan: &QueryPlan,
            policy: Option<QueryPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            validate_plan_matches_statement(&statement._as, &plan._as)?;
            let policy = policy.map(|p| p._as.clone()).unwrap_or_default();
            let client = self._as.clone();
            let stmt = statement._as.clone();
            let core_plan = plan._as.clone();
            let bridge = self.require_bridge()?;
            let recordset_bridge = bridge.clone();

            completion::batched_future_into_py(bridge, py, async move {
                let res = client
                    .query_with_plan(&policy, partition_filter._as, stmt, core_plan)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(Recordset {
                    _as: res,
                    _stream: Arc::new(Mutex::new(None)),
                    bridge: Some(recordset_bridge),
                })
            })
        }

        /// Creates a new user with password and roles. Clear-text password will be hashed using bcrypt
        /// before sending to server.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (user, password, roles, *, policy = None))]
        pub fn create_user<'a>(
            &self,
            user: String,
            password: String,
            roles: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let roles: Vec<&str> = roles.iter().map(|r| &**r).collect();
                client
                    .create_user(&admin_policy, &user, &password, &roles)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Creates a PKI user with roles. PKI users authenticate via TLS client certificate (no password).
        /// Supported by Aerospike Server v8.1+ Enterprise.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (user, roles, *, policy = None))]
        pub fn create_pki_user<'a>(
            &self,
            user: String,
            roles: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let roles: Vec<&str> = roles.iter().map(|r| &**r).collect();
                client
                    .create_pki_user(&admin_policy, &user, &roles)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Removes a user from the cluster.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (user, *, policy = None))]
        pub fn drop_user<'a>(&self, user: String, policy: Option<AdminPolicy>, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .drop_user(&admin_policy, &user)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Changes a user's password. Clear-text password will be hashed using bcrypt before sending to server.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (user, password, *, policy = None))]
        pub fn change_password<'a>(
            &self,
            user: String,
            password: String,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .change_password(&admin_policy, &user, &password)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Adds roles to user's list of roles.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (user, roles, *, policy = None))]
        pub fn grant_roles<'a>(
            &self,
            user: String,
            roles: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let roles: Vec<&str> = roles.iter().map(|r| &**r).collect();
                client
                    .grant_roles(&admin_policy, &user, &roles)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Removes roles from user's list of roles.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (user, roles, *, policy = None))]
        pub fn revoke_roles<'a>(
            &self,
            user: String,
            roles: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let roles: Vec<&str> = roles.iter().map(|r| &**r).collect();
                client
                    .revoke_roles(&admin_policy, &user, &roles)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Retrieves users and their roles.
        /// If None is passed for the user argument, all users will be returned.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (user = None, *, policy = None))]
        pub fn query_users<'a>(
            &self,
            user: Option<String>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let user = user.as_deref();
                let res = client
                    .query_users(&admin_policy, user)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                let res: Vec<User> = res.iter().map(|u| User { _as: u.clone() }).collect();
                Ok(res)
            })
        }

        /// Retrieves roles and their privileges.
        /// If None is passed for the role argument, all roles will be returned.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (role = None, *, policy = None))]
        pub fn query_roles<'a>(
            &self,
            role: Option<String>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let role: Option<&str> = role.as_deref();
                let res = client
                    .query_roles(&admin_policy, role)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                let res: Vec<Role> = res.iter().map(|r| Role { _as: r.clone() }).collect();
                Ok(res)
            })
        }

        /// Creates a user-defined role.
        /// Quotas require server security configuration "enable-quotas" to be set to true.
        /// Pass 0 for quota values for no limit.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (role_name, privileges, allowlist, read_quota, write_quota, *, policy = None))]
        pub fn create_role<'a>(
            &self,
            role_name: String,
            privileges: Vec<Privilege>,
            allowlist: Vec<String>,
            read_quota: u32,
            write_quota: u32,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let allowlist: Vec<&str> = allowlist.iter().map(|al| &**al).collect();
                let privileges: Vec<aerospike_core::Privilege> =
                    privileges.iter().map(|r| r._as.clone()).collect();
                client
                    .create_role(&admin_policy, &role_name, &privileges, &allowlist, read_quota, write_quota)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Removes a user-defined role.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (role_name, *, policy = None))]
        pub fn drop_role<'a>(
            &self,
            role_name: String,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .drop_role(&admin_policy, &role_name)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Grants privileges to a user-defined role.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (role_name, privileges, *, policy = None))]
        pub fn grant_privileges<'a>(
            &self,
            role_name: String,
            privileges: Vec<Privilege>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let privileges: Vec<aerospike_core::Privilege> =
                    privileges.iter().map(|p| p._as.clone()).collect();
                client
                    .grant_privileges(&admin_policy, &role_name, &privileges)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Revokes privileges from a user-defined role.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (role_name, privileges, *, policy = None))]
        pub fn revoke_privileges<'a>(
            &self,
            role_name: String,
            privileges: Vec<Privilege>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let privileges: Vec<aerospike_core::Privilege> =
                    privileges.iter().map(|p| p._as.clone()).collect();
                client
                    .revoke_privileges(&admin_policy, &role_name, &privileges)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Sets IP address allowlist for a role.
        /// If allowlist is nil or empty, it removes existing allowlist from role.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (role_name, allowlist, *, policy = None))]
        pub fn set_allowlist<'a>(
            &self,
            role_name: String,
            allowlist: Vec<String>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let allowlist: Vec<&str> = allowlist.iter().map(|al| &**al).collect();
                client
                    .set_allowlist(&admin_policy, &role_name, &allowlist)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        /// Sets maximum reads/writes per second limits for a role.
        /// If a quota is zero, the limit is removed.
        /// Quotas require server security configuration "enable-quotas" to be set to true.
        /// Pass 0 for quota values for no limit.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (role_name, read_quota, write_quota, *, policy = None))]
        pub fn set_quotas<'a>(
            &self,
            role_name: String,
            read_quota: u32,
            write_quota: u32,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy = policy.map(|p| p._as).unwrap_or_default();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .set_quotas(&admin_policy, &role_name, read_quota, write_quota)
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(None::<bool>)
            })
        }

        fn __str__(&self) -> PyResult<String> {
            Ok(self.seeds.to_string())
        }

        fn __repr__(&self) -> PyResult<String> {
            let s = self.__str__()?;
            Ok(format!("Client('{}')", s))
        }

        pub fn __copy__(&self) -> Self {
            self.clone()
        }

        pub fn __deepcopy__(&self, _memo: &Bound<PyDict>) -> Self {
            // fast bitwise copy instead of python's pickling process
            self.clone()
        }

        /// Returns a list of the names of the active server nodes in the cluster.
        ///
        /// Reads the tended node list (no network I/O), so it is a plain
        /// method callable from both async and sync code.
        pub fn node_names(&self) -> Vec<String> {
            self._as.node_names()
        }

        /// Return node given its name.
        ///
        /// Reads the tended node list (no network I/O), so it is a plain
        /// method callable from both async and sync code.
        pub fn get_node(&self, name: &str) -> PyResult<Node> {
            let node = self._as.get_node(name).map_err(|e| PyErr::from(RustClientError(e)))?;
            Ok(Node { _as: node })
        }

        /// Returns a list of all active server nodes in the cluster.
        ///
        /// Reads the tended node list (no network I/O), so it is a plain
        /// method callable from both async and sync code.
        pub fn nodes(&self) -> Vec<Node> {
            self._as.nodes().into_iter().map(|n| Node { _as: n }).collect()
        }

        /// Commit a multi-record transaction.
        ///
        /// Verifies all transaction record versions, then applies all writes
        /// atomically. Returns a :class:`CommitStatus` indicating the outcome
        /// (``OK``, ``ALREADY_COMMITTED``, or ``CLOSE_ABANDONED``). An
        /// abandoned roll-forward raises
        /// :exc:`aerospike_native.exceptions.CommitFailedError` with
        /// ``commit_error_type=ROLL_FORWARD_ABANDONED`` rather than returning
        /// that status: the writes are not yet visible, and returning
        /// normally would present them as committed. The triggering
        /// timeout or server code is on the exception.
        ///
        /// Args:
        ///     txn: The transaction to commit.
        ///     verify_policy: Policy for the verify phase. ``None`` (the
        ///         default) applies the default verify policy.
        ///     roll_policy: Policy for the roll-forward phase. ``None`` (the
        ///         default) applies the default roll policy.
        ///
        /// Returns:
        ///     CommitStatus: The outcome of the commit.
        ///
        /// Raises:
        ///     CommitFailedError: If the transaction could not be committed,
        ///         including when roll-forward was abandoned.
        ///
        /// Example::
        ///
        ///     status = await client.commit(txn)
        ///     assert status == CommitStatus.OK
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[CommitStatus]", imports=("typing")))]
        #[pyo3(signature = (txn, *, verify_policy=None, roll_policy=None))]
        pub fn commit<'a>(
            &self,
            txn: &Txn,
            verify_policy: Option<TxnVerifyPolicy>,
            roll_policy: Option<TxnRollPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let txn_arc = txn._as.clone();
            // Route through the explicit-policy entry only when a caller
            // supplied one, so the no-policy call keeps core's defaults.
            let policies = match (verify_policy, roll_policy) {
                (None, None) => None,
                (verify, roll) => Some((
                    verify.map(|p| p._as).unwrap_or_default(),
                    roll.map(|p| p._as).unwrap_or_default(),
                )),
            };
            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let status = match &policies {
                    Some((verify, roll)) => {
                        client.commit_with_policies(verify, roll, &txn_arc).await
                    }
                    None => client.commit(&txn_arc).await,
                }
                .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(CommitStatus::from(status))
            })
        }

        /// Abort a multi-record transaction, rolling back all writes.
        ///
        /// Args:
        ///     txn: The transaction to abort.
        ///     roll_policy: Policy for the roll-back phase. ``None`` (the
        ///         default) applies the default roll policy.
        ///
        /// Returns:
        ///     AbortStatus: The outcome of the abort.
        ///
        /// Raises:
        ///     AerospikeError: If the abort itself encounters a server error.
        ///
        /// Example::
        ///
        ///     status = await client.abort(txn)
        ///     assert status == AbortStatus.OK
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[AbortStatus]", imports=("typing")))]
        #[pyo3(signature = (txn, *, roll_policy=None))]
        pub fn abort<'a>(
            &self,
            txn: &Txn,
            roll_policy: Option<TxnRollPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let txn_arc = txn._as.clone();
            let roll_policy = roll_policy.map(|p| p._as);
            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let status = match &roll_policy {
                    Some(roll) => client.abort_with_policy(roll, &txn_arc).await,
                    None => client.abort(&txn_arc).await,
                }
                .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(AbortStatus::from(status))
            })
        }

        /// Execute an info command on a random cluster node.
        ///
        /// Args:
        ///     command: The info command to execute (e.g., "namespaces", "statistics", "build").
        ///
        /// Returns:
        ///     A dictionary containing the info command response as key-value pairs.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Dict[str, str]]", imports=("typing")))]
        pub fn info<'a>(&self, command: String, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let node = client
                    
                    .random_node()
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                let policy = aerospike_core::AdminPolicy::default();
                let response = node
                    .info(&policy, &[&command])
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;

                Ok(response)
            })
        }

        /// Execute an info command on all cluster nodes.
        ///
        /// Args:
        ///     command: The info command to execute (e.g., "namespaces", "statistics", "build").
        ///
        /// Returns:
        ///     A dictionary mapping node names to their info command responses.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Dict[str, typing.Dict[str, str]]]", imports=("typing")))]
        pub fn info_on_all_nodes<'a>(&self, command: String, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                let nodes = client
                    .nodes();

                let mut results: HashMap<String, IndexMap<String, String>> = HashMap::new();

                for node in nodes {
                    let node_name = node.name().to_string();
                    let policy = aerospike_core::AdminPolicy::default();
                    match node.info(&policy, &[&command]).await {
                        Ok(response) => {
                            results.insert(node_name, response);
                        }
                        Err(e) => {
                            // Log error but continue with other nodes
                            // We could also collect errors, but for now just skip failed nodes
                            eprintln!("Failed to get info from node {}: {}", node_name, e);
                        }
                    }
                }

                Ok(results)
            })
        }

        // -- Metrics ---------------------------------------------------------
        //
        // Collection, draining and aggregation all live in the core and are
        // synchronous there, so these are plain methods on the async client —
        // no awaitable, usable on blocking-constructed clients too.

        /// Enable metrics collection. Defaults to :class:`MetricsPolicy`'s
        /// milliseconds/7-column scheme recording every command when no
        /// policy is given.
        ///
        /// Re-enabling with a changed latency unit or histogram shape discards
        /// the accumulated latency samples; counters are always retained.
        #[pyo3(signature = (policy = None))]
        pub fn enable_metrics(&self, policy: Option<MetricsPolicy>) {
            self._as.enable_metrics(policy.unwrap_or_default()._as);
        }

        /// Disable metrics collection. Accumulated data is retained.
        pub fn disable_metrics(&self) {
            self._as.disable_metrics();
        }

        /// Whether metrics collection is currently enabled.
        pub fn metrics_enabled(&self) -> bool {
            self._as.metrics_enabled()
        }

        /// Snapshot cluster metrics. Values are cumulative since metrics were
        /// enabled (gauges are point-in-time); drains and aggregates per-node
        /// state, so poll at an export interval rather than per operation.
        pub fn metrics(&self, py: Python<'_>) -> ClusterMetrics {
            let client = self._as.clone();
            py.detach(move || ClusterMetrics { _as: client.metrics() })
        }

        /// Sets XDR filter for given datacenter and namespace. Pass None as filter to remove.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
        #[pyo3(signature = (datacenter, namespace, filter_expression = None, *, policy = None))]
        pub fn set_xdr_filter<'a>(
            &self,
            datacenter: String,
            namespace: String,
            filter_expression: Option<FilterExpression>,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let client = self._as.clone();
            let admin_policy =
                policy.map(|p| p._as).unwrap_or_default();
            let expr = filter_expression.clone();

            completion::batched_future_into_py(self.require_bridge()?, py, async move {
                client
                    .set_xdr_filter(
                        &admin_policy,
                        &datacenter,
                        &namespace,
                        expr.as_ref().map(|e| &e._as),
                    )
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(None::<bool>)
            })
        }
    }

/// Return a null value for use in Aerospike operations.
/// This is equivalent to Python None but represents an Aerospike null value.
#[pyfunction]
#[gen_stub_pyfunction(module = "_native")]
pub fn null(py: Python) -> Bound<PyAny> {
    py.None().into_bound(py)
}

/// Inspect a Python list of `Operation` / `ExpOperation` / CDT / HLL ops and
/// report whether any are *writes* (so the caller can choose between
/// :class:`BatchReadOp` and :class:`BatchWriteOp` for the heterogeneous
/// :meth:`Client.batch_stream` ops list).
///
/// PSDK uses this from its streaming-batch builder to match the wire
/// dispatch that the buffered :meth:`Client.batch_operate` does internally
/// — a key-op-list with only reads (e.g. an AEL `select_from` expression)
/// must land as `BatchOperation::read_ops`, not `BatchOperation::write`,
/// or the server rejects the per-node group.
#[pyfunction]
#[gen_stub_pyfunction(module = "_native")]
pub fn has_any_write_op(py: Python<'_>, operations: Vec<Py<PyAny>>) -> PyResult<bool> {
    let owcs = extract_py_ops_with_ctx(py, &operations)?;
    let (_core_ops, has_write) = convert_ops_with_ctx_to_core(&owcs, false)?;
    Ok(has_write)
}

// Resolved once per process: `geojson()` can run per value on geo workloads,
// and a per-call import convoys free-threaded builds on the `sys.modules` lock.
static JSON_LOADS: pyo3::sync::PyOnceLock<Py<PyAny>> = pyo3::sync::PyOnceLock::new();

/// Convert a GeoJSON string or coordinate pair to a GeoJSON object.
///
/// Accepts:
/// - GeoJSON JSON string: '{"type": "Point", "coordinates": [-122.0, 37.0]}'
/// - Coordinate pair string: "-122.0, 37.5" (longitude, latitude)
#[pyfunction]
#[gen_stub_pyfunction(module = "_native")]
pub fn geojson<'a>(py: Python<'a>, geo_str: &str) -> PyResult<GeoJSON> {
    // First, try to parse as GeoJSON JSON string
    // Check if it looks like JSON (starts with '{' and contains "type")
    if geo_str.trim_start().starts_with('{') && geo_str.contains("\"type\"") {
        // Try to parse as JSON and create GeoJSON from it
        let json_loads = JSON_LOADS.import(py, "json", "loads")?;
        let geo_dict = json_loads.call1((geo_str,))?;

        // Use GeoJSON constructor which accepts dict
        return GeoJSON::new(py, geo_dict.into_bound_py_any(py)?.as_any());
    }

    // Otherwise, try to parse as coordinate pair string like "122.0, 37.5"
    let parts: Vec<&str> = geo_str.split(',').map(|s| s.trim()).collect();
    if parts.len() != 2 {
        return Err(PyValueError::new_err(
            format!("Invalid input: '{}'. Expected GeoJSON JSON string or coordinate pair 'longitude, latitude'", geo_str)
        ));
    }

    let lng: f64 = parts[0].parse()
        .map_err(|_| PyValueError::new_err(format!("Invalid longitude: '{}'", parts[0])))?;
    let lat: f64 = parts[1].parse()
        .map_err(|_| PyValueError::new_err(format!("Invalid latitude: '{}'", parts[1])))?;

    // Create GeoJSON Point structure
    let point_dict = PyDict::new(py);
    point_dict.set_item("type", "Point")?;
    // Create coordinates list [lng, lat]
    let coords_vec = vec![lng, lat];
    point_dict.set_item("coordinates", coords_vec)?;

    // Use GeoJSON constructor to create from dict
    GeoJSON::new(py, point_dict.as_any())
}

/// Logger that forwards to Python's `logging` via pyo3-log when the interpreter
/// is available, falling back to stderr for messages emitted on background tokio
/// threads after Python shutdown.
struct ResilientPyLogger {
    inner: pyo3_log::Logger,
}

impl log::Log for ResilientPyLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if unsafe { pyo3::ffi::Py_IsInitialized() } != 0 {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.inner.log(record);
            }));
        }
        // Silently drop messages when Python is unavailable (shutdown).
    }

    fn flush(&self) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.inner.flush();
        }));
    }
}

/// Cache-reset handle for the pyo3-log bridge. `Caching::LoggersAndLevels`
/// snapshots each logger's effective level on first use; Python-side
/// `setLevel()` calls made after that are invisible to Rust-emitted records
/// until the cache is cleared through this handle.
static LOG_RESET_HANDLE: std::sync::OnceLock<pyo3_log::ResetHandle> = std::sync::OnceLock::new();

/// Re-sync Rust-emitted log levels with the Python `logging` hierarchy.
///
/// The bridge that forwards Rust log records to Python's `logging` module
/// caches each logger's effective level the first time that logger emits.
/// Calling `logging.getLogger("aerospike_core.cluster").setLevel(...)` at
/// runtime therefore has no effect on Rust-emitted records for
/// already-cached loggers until this function is called to drop the cache.
/// Cheap; safe to call from any thread.
#[pyfunction]
#[gen_stub_pyfunction(module = "_native")]
pub fn refresh_log_levels() {
    if let Some(handle) = LOG_RESET_HANDLE.get() {
        handle.reset();
    }
}

#[pymodule(gil_used = false)]
fn _native(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Filter the noisy default panic-hook output for a specific class of
    // shutdown-race panic: Tokio worker threads call `Python::attach` while
    // the interpreter is finalizing (typically because a user-code exception
    // escaped `asyncio.run()` while in-flight async ops were still spawned).
    // pyo3 asserts on `Py_IsInitialized() != 0` and panics; the panic
    // unwinds harmlessly (Tokio doesn't abort the process on worker
    // panics) and our completion bridge's `catch_unwind` swallows the
    // unwind, but the default hook prints the assertion to stderr *before*
    // unwind starts.  That stderr noise looks like a crash but isn't.
    // Filter it; leave every other panic alone.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let msg_str = payload
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| payload.downcast_ref::<&'static str>().copied());
        if let Some(s) = msg_str {
            // Message text differs by pyo3 version: pre-0.29 asserted
            // "Python interpreter is not initialized"; pyo3 0.29+ rejects the
            // attach with "Cannot attach to the Python interpreter while it is
            // finalizing".  Match both so the filter survives pyo3 bumps.
            if s.contains("Python interpreter is not initialized")
                || s.contains("Cannot attach to the Python interpreter while it is finalizing")
            {
                return;
            }
        }
        prev_hook(info);
    }));

    // Configure the pyo3-async-runtimes Tokio runtime.  Must precede any
    // future_into_py / batched_future_into_py call — pyo3-async-runtimes
    // builds the runtime lazily on first get_runtime(), so as long as we
    // install our builder before user code runs, the override applies.
    // Reads AEROSPIKE_PNC_RUNTIME_WORKERS for an optional worker-count
    // override; defaults to Tokio's available_parallelism otherwise.
    runtime::init();

    // Bridge Rust `log` records to Python's `logging` module.
    // Rust module paths become Python logger names, e.g.
    //   aerospike_core::cluster -> logging.getLogger("aerospike_core.cluster")
    // Uses ResilientPyLogger to avoid panics on background threads during shutdown.
    let inner = pyo3_log::Logger::new(py, pyo3_log::Caching::LoggersAndLevels)?;
    let _ = LOG_RESET_HANDLE.set(inner.reset_handle());
    let logger = ResilientPyLogger { inner };
    log::set_max_level(log::LevelFilter::Debug);
    let _ = log::set_logger(Box::leak(Box::new(logger)));
    log::debug!(target: "aerospike_native", "pyo3-log bridge active");

    // Add all main classes to the top level for easy importing
    m.add_class::<Client>()?;
    m.add_class::<QueryDuration>()?;
    m.add_class::<Replica>()?;
    m.add_class::<Concurrency>()?;
    m.add_class::<Expiration>()?;
    m.add_class::<CommitLevel>()?;
    m.add_class::<TxnState>()?;
    m.add_class::<CommitStatus>()?;
    m.add_class::<AbortStatus>()?;
    m.add_class::<CommitErrorType>()?;
    m.add_class::<LoopVarPart>()?;
    m.add_class::<SelectFlags>()?;
    m.add_class::<ModifyFlags>()?;
    m.add_class::<Txn>()?;
    m.add_class::<FastRng>()?;
    m.add_class::<ReadModeAP>()?;
    m.add_class::<ReadModeSC>()?;
    m.add_class::<RecordExistsAction>()?;
    m.add_class::<GenerationPolicy>()?;
    m.add_class::<IndexType>()?;
    m.add_class::<CollectionIndexType>()?;
    m.add_class::<PrivilegeCode>()?;
    m.add_class::<Privilege>()?;
    m.add_class::<ResultCode>()?;
    m.add_class::<SubCode>()?;
    m.add_class::<ErrorDetailVerbosity>()?;
    m.add_class::<ExpressionTrace>()?;

    m.add_class::<List>()?;
    m.add_class::<Map>()?;
    m.add_class::<SortedMap>()?;
    m.add_class::<Blob>()?;
    m.add_class::<GeoJSON>()?;
    m.add_class::<HLL>()?;
    m.add_class::<Vector>()?;
    m.add_class::<VectorElementType>()?;

    m.add_class::<Key>()?;
    m.add_class::<Record>()?;
    m.add_class::<Recordset>()?;
    m.add_class::<Filter>()?;
    m.add_class::<Statement>()?;
    m.add_class::<OrderBy>()?;
    m.add_class::<BinDataType>()?;
    m.add_class::<Order>()?;
    m.add_class::<ExpType>()?;
    m.add_class::<VectorDistanceMetric>()?;
    m.add_class::<FilterExpression>()?;
    m.add_class::<ServerError>()?;
    m.add_class::<Operation>()?;
    m.add_class::<ListOperation>()?;
    m.add_class::<MapOperation>()?;
    m.add_class::<CTX>()?;
    m.add_class::<ListReturnType>()?;
    m.add_class::<ListSortFlags>()?;
    m.add_class::<BitOperation>()?;
    m.add_class::<HllOperation>()?;
    m.add_class::<HLLWriteFlags>()?;
    m.add_class::<HLLPolicy>()?;
    m.add_class::<ExpOperation>()?;
    m.add_class::<ExpWriteFlags>()?;
    m.add_class::<ExpReadFlags>()?;
    m.add_class::<RegexFlag>()?;
    m.add_class::<CdtOperation>()?;

    m.add_class::<BasePolicy>()?;
    m.add_class::<AdminPolicy>()?;
    m.add_class::<ReadPolicy>()?;

    // Add helper functions
    m.add_function(wrap_pyfunction!(null, m)?)?;
    m.add_function(wrap_pyfunction!(has_any_write_op, m)?)?;
    m.add_function(wrap_pyfunction!(geojson, m)?)?;
    m.add_function(wrap_pyfunction!(refresh_log_levels, m)?)?;
    m.add_class::<AuthMode>()?;
    m.add_class::<ClientPolicy>()?;
    m.add_class::<WritePolicy>()?;
    m.add_class::<QueryPolicy>()?;
    m.add_class::<BatchRecord>()?;
    m.add_class::<BatchRecordStream>()?;
    m.add_class::<BatchPolicy>()?;
    m.add_class::<BatchReadPolicy>()?;
    m.add_class::<BatchWritePolicy>()?;
    m.add_class::<BatchDeletePolicy>()?;
    m.add_class::<BatchUDFPolicy>()?;
    m.add_class::<TxnVerifyPolicy>()?;
    m.add_class::<TxnRollPolicy>()?;
    m.add_class::<BatchReadOp>()?;
    m.add_class::<BatchWriteOp>()?;
    m.add_class::<BatchDeleteOp>()?;
    m.add_class::<BatchUDFOp>()?;
    m.add_class::<ListOrderType>()?;
    m.add_class::<ListWriteFlags>()?;
    m.add_class::<ListPolicy>()?;
    m.add_class::<MapOrder>()?;
    m.add_class::<MapWriteMode>()?;
    m.add_class::<MapWriteFlags>()?;
    m.add_class::<MapReturnType>()?;
    m.add_class::<SpecialValue>()?;
    m.add_class::<MapPolicy>()?;
    m.add_class::<BitwiseResizeFlags>()?;
    m.add_class::<BitWriteFlags>()?;
    m.add_class::<BitwiseOverflowActions>()?;
    m.add_class::<BitPolicy>()?;
    m.add_class::<StringWriteFlags>()?;
    m.add_class::<StringRegexFlags>()?;
    m.add_class::<StringNumericType>()?;
    m.add_class::<StringOperation>()?;
    m.add_class::<PartitionStatus>()?;
    m.add_class::<PartitionFilter>()?;
    m.add_class::<QueryPlan>()?;
    m.add_class::<QuerySelection>()?;
    m.add_class::<QueryWhereFlags>()?;
    m.add_class::<UDFLang>()?;
    m.add_class::<TaskStatus>()?;
    m.add_class::<RegisterTask>()?;
    m.add_class::<UdfRemoveTask>()?;
    m.add_class::<IndexTask>()?;
    m.add_class::<DropIndexTask>()?;
    m.add_class::<ExecuteTask>()?;
    m.add_class::<Version>()?;
    m.add_class::<Node>()?;
    m.add_class::<MetricsPolicy>()?;
    m.add_class::<Sampler>()?;
    m.add_class::<LatencyUnit>()?;
    m.add_class::<CommandType>()?;
    m.add_class::<ClusterMetrics>()?;
    m.add_class::<NodeMetricsSnapshot>()?;
    m.add_class::<CommandMetric>()?;
    m.add_class::<Histogram>()?;
    #[cfg(feature = "tls")]
    m.add_class::<TlsConfig>()?;
    m.add_class::<Host>()?;

    m.add_function(wrap_pyfunction!(new_client, m)?)?;
    m.add_function(wrap_pyfunction!(crate::blocking::new_client_blocking, m)?)?;
    m.add_class::<LocalClient>()?;
    m.add_class::<completion::CompletionDrainer>()?;

    // Create and register the exceptions submodule
    // Exceptions are only available via aerospike_native.exceptions submodule
    // They are not exposed at the top level to avoid namespace pollution
    let exceptions_module = PyModule::new(py, "exceptions")?;
    exceptions_module.add("AerospikeError", py.get_type::<AerospikeError>())?;
    exceptions_module.add("ServerError", py.get_type::<ServerError>())?;
    exceptions_module.add("UDFBadResponse", py.get_type::<UDFBadResponse>())?;
    exceptions_module.add("TimeoutError", py.get_type::<TimeoutError>())?;
    exceptions_module.add("BadResponse", py.get_type::<BadResponse>())?;
    exceptions_module.add("ConnectionError", py.get_type::<ConnectionError>())?;
    exceptions_module.add("InvalidNodeError", py.get_type::<InvalidNodeError>())?;
    exceptions_module.add("InvalidNamespaceError", py.get_type::<InvalidNamespaceError>())?;
    exceptions_module.add("NoMoreConnections", py.get_type::<NoMoreConnections>())?;
    exceptions_module.add("RecvError", py.get_type::<RecvError>())?;
    exceptions_module.add("Base64DecodeError", py.get_type::<Base64DecodeError>())?;
    exceptions_module.add("InvalidUTF8", py.get_type::<InvalidUTF8>())?;
    exceptions_module.add("ParseAddressError", py.get_type::<ParseAddressError>())?;
    exceptions_module.add("ParseIntError", py.get_type::<ParseIntError>())?;
    exceptions_module.add("ValueError", py.get_type::<ValueError>())?;
    exceptions_module.add("IoError", py.get_type::<IoError>())?;
    exceptions_module.add("PasswordHashError", py.get_type::<PasswordHashError>())?;
    exceptions_module.add("InvalidRustClientArgs", py.get_type::<InvalidRustClientArgs>())?;
    exceptions_module.add("ClientError", py.get_type::<ClientError>())?;
    exceptions_module.add("CommitFailedError", py.get_type::<CommitFailedError>())?;
    exceptions_module.add("BatchFailedError", py.get_type::<BatchFailedError>())?;
    exceptions_module.add("MaxErrorRate", py.get_type::<MaxErrorRate>())?;
    exceptions_module.add("ResultCode", py.get_type::<ResultCode>())?;
    m.add_submodule(&exceptions_module)?;

    Ok(())
}
