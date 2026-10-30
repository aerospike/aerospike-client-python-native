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

use std::marker::PhantomData;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::PyErrArguments;
use pyo3::PyTypeInfo;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use aerospike_core::errors::{Error, ErrorKind};
use aerospike_core::ResultCode as CoreResultCode;

use crate::enums::ResultCode;
use crate::server_error::ExpressionTrace;

create_exception!(aerospike_native.exceptions, AerospikeError, pyo3::exceptions::PyException);

// Server-related exceptions
// ServerError is a custom exception with a result_code property
// Note: It extends PyException directly, but Python-side it should be treated as an AerospikeError subclass
#[gen_stub_pyclass(module = "_native")]
#[pyclass(extends = PyException, subclass)]
pub struct ServerError {
    result_code: CoreResultCode,
    in_doubt: bool,
    sub_code: Option<u32>,
    server_message: Option<String>,
    exp_trace: Option<ExpressionTrace>,
    node: Option<String>,
    iteration: Option<u32>,
    base_message: Option<String>,
    sub_exceptions: Option<Py<PyAny>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl ServerError {
    #[new]
    #[pyo3(signature = (_message, result_code, in_doubt=false, sub_code=None, server_message=None, exp_trace=None, node=None, iteration=None, base_message=None, sub_exceptions=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        _message: String,
        result_code: ResultCode,
        in_doubt: bool,
        sub_code: Option<u32>,
        server_message: Option<String>,
        exp_trace: Option<ExpressionTrace>,
        node: Option<String>,
        iteration: Option<u32>,
        base_message: Option<String>,
        sub_exceptions: Option<Py<PyAny>>,
    ) -> PyResult<Self> {
        // Note: message is handled by the base PyException; the fields here
        // are the structured accessors.
        let Some(result_code) = result_code.server() else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "ServerError takes a code the server returns; client-side \
                 codes belong on the client-side exception types",
            ));
        };
        Ok(ServerError {
            result_code,
            in_doubt,
            sub_code,
            server_message,
            exp_trace,
            node,
            iteration,
            base_message,
            sub_exceptions,
        })
    }

    #[getter]
    fn result_code(&self) -> ResultCode {
        ResultCode::from(self.result_code)
    }

    #[getter]
    fn in_doubt(&self) -> bool {
        self.in_doubt
    }

    /// Server-supplied error subcode, present when the request asked for
    /// extended error detail (``error_detail_verbosity`` >= 1) and the
    /// server (>= 8.2.0) attached one. Subcode values are scoped to their
    /// parent result code — interpret the (result_code, sub_code) pair.
    #[getter]
    fn sub_code(&self) -> Option<u32> {
        self.sub_code
    }

    /// Server-supplied error detail message, present at
    /// ``error_detail_verbosity`` >= 2 when the server attached one.
    #[getter]
    fn server_message(&self) -> Option<String> {
        self.server_message.clone()
    }

    /// Structured server-supplied expression build trace, present only at
    /// ``error_detail_verbosity`` 3 on an expression build failure and when the
    /// server build emits one.
    #[getter]
    fn exp_trace(&self) -> Option<ExpressionTrace> {
        self.exp_trace.clone()
    }

    /// Last node the command was attempted on, when the retry loop recorded
    /// one. ``None`` for failures that never reached node selection.
    #[getter]
    fn node(&self) -> Option<String> {
        self.node.clone()
    }

    /// Number of attempts before the command failed, when the retry loop
    /// recorded it. ``None`` when the failure precedes the retry loop.
    #[getter]
    fn iteration(&self) -> Option<u32> {
        self.iteration
    }

    /// The failure message without the retry-context decoration that the
    /// full exception message carries. ``None`` when no decorated message
    /// was recorded.
    #[getter]
    fn base_message(&self) -> Option<String> {
        self.base_message.clone()
    }

    /// Exceptions from prior retry attempts of the same command, oldest
    /// first. ``None`` when the command was not retried.
    #[getter]
    fn sub_exceptions(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.sub_exceptions.as_ref().map(|list| list.clone_ref(py))
    }

    /// ``str(exc)`` is the human-readable message alone. Without this, the
    /// PyException base renders the whole constructor args tuple (message,
    /// result_code, in_doubt, ...) — every field of which is already a
    /// structured accessor.
    fn __str__(slf: &pyo3::Bound<'_, Self>) -> PyResult<String> {
        let args = slf.getattr("args")?;
        match args.get_item(0) {
            Ok(first) => first.str().map(|s| s.to_string()),
            Err(_) => Ok(String::new()),
        }
    }
}

// `aerospike_native.exceptions._get_server_error_class`, resolved once per
// process: this runs on every server error, and a per-call `py.import`
// convoys free-threaded builds on the import mutex under an error storm.
static GET_SERVER_ERROR_CLASS: pyo3::sync::PyOnceLock<Py<PyAny>> =
    pyo3::sync::PyOnceLock::new();

// Resolve the Python ServerError subclass for the given result code (for dispatch).
fn resolve_server_error_class(py: Python<'_>, result_code: CoreResultCode) -> PyResult<pyo3::Bound<'_, pyo3::types::PyAny>> {
    let func = GET_SERVER_ERROR_CLASS.get_or_try_init(py, || {
        Ok::<_, PyErr>(
            py.import("aerospike_native.exceptions")?
                .getattr("_get_server_error_class")?
                .unbind(),
        )
    })?;
    let rc_wrapper = ResultCode::from(result_code);
    let py_rc = Py::new(py, rc_wrapper)?;
    func.bind(py).call1((py_rc,))
}

// Deferred arguments for a ServerError (or subclass) PyErr.  Holds only plain
// data (Send + Sync + 'static) so it can be carried across threads without
// touching Python.  `detail` carries the extended server error detail
// (subcode / message / expression trace) when the server attached one.
struct ServerErrorArgs {
    message: String,
    result_code: CoreResultCode,
    in_doubt: bool,
    detail: Option<aerospike_core::ServerErrorDetail>,
    ctx: RetryContext,
}

// Tag for the client-side exception type a sub-error maps to.  Captured at
// the boundary because `ErrorKind` itself is neither `Clone` nor cheap to
// carry, while the deferred-arguments structs must stay plain Send + Sync
// data.
enum ClientKindTag {
    Timeout,
    Connection,
    NoMoreConnections,
    MaxErrorRate,
    InvalidNode,
    InvalidNamespace,
    InvalidArgument,
    BadResponse,
    UdfBadResponse,
    Client,
}

impl ClientKindTag {
    fn from_kind(kind: &ErrorKind) -> Self {
        match kind {
            ErrorKind::Timeout => ClientKindTag::Timeout,
            ErrorKind::Connection | ErrorKind::ConnectionPoolEmpty => ClientKindTag::Connection,
            ErrorKind::NoMoreConnections => ClientKindTag::NoMoreConnections,
            ErrorKind::MaxErrorRate => ClientKindTag::MaxErrorRate,
            ErrorKind::InvalidNode => ClientKindTag::InvalidNode,
            ErrorKind::InvalidNamespace => ClientKindTag::InvalidNamespace,
            ErrorKind::InvalidArgument => ClientKindTag::InvalidArgument,
            ErrorKind::BadResponse | ErrorKind::ParsePeers => ClientKindTag::BadResponse,
            ErrorKind::UdfBadResponse => ClientKindTag::UdfBadResponse,
            _ => ClientKindTag::Client,
        }
    }

    fn type_object<'py>(&self, py: Python<'py>) -> pyo3::Bound<'py, pyo3::types::PyType> {
        match self {
            ClientKindTag::Timeout => py.get_type::<TimeoutError>(),
            ClientKindTag::Connection => py.get_type::<ConnectionError>(),
            ClientKindTag::NoMoreConnections => py.get_type::<NoMoreConnections>(),
            ClientKindTag::MaxErrorRate => py.get_type::<MaxErrorRate>(),
            ClientKindTag::InvalidNode => py.get_type::<InvalidNodeError>(),
            ClientKindTag::InvalidNamespace => py.get_type::<InvalidNamespaceError>(),
            ClientKindTag::InvalidArgument => py.get_type::<ValueError>(),
            ClientKindTag::BadResponse => py.get_type::<BadResponse>(),
            ClientKindTag::UdfBadResponse => py.get_type::<UDFBadResponse>(),
            ClientKindTag::Client => py.get_type::<ClientError>(),
        }
    }
}

// Plain-data snapshot of one prior-attempt error.  Core's `Error` is not
// `Clone`, so the fields the Python surface needs are copied out while the
// original error is still on hand; materialization happens later in
// `arguments()` on the event-loop/drainer thread.
struct SubErrorData {
    message: String,
    result_code: Option<CoreResultCode>,
    // The flat code for a sub-error that carries no server code.
    code: i32,
    in_doubt: bool,
    node: Option<String>,
    iteration: Option<u32>,
    detail: Option<aerospike_core::ServerErrorDetail>,
    kind: ClientKindTag,
}

fn capture_sub_error(e: &Error) -> SubErrorData {
    SubErrorData {
        message: e.to_string(),
        result_code: e.server_result_code(),
        code: e.result_code(),
        in_doubt: e.in_doubt(),
        node: e.node().map(str::to_string),
        iteration: e.iteration(),
        detail: e.server_error_detail().cloned(),
        kind: ClientKindTag::from_kind(e.kind()),
    }
}

// Retry/diagnostic context shared by the server and client error paths:
// where the command failed (node), how many attempts it took (iteration),
// the undecorated message, and the errors of prior attempts.
struct RetryContext {
    node: Option<String>,
    iteration: Option<u32>,
    base_message: String,
    subs: Vec<SubErrorData>,
}

fn capture_retry_context(err: &Error) -> RetryContext {
    RetryContext {
        node: err.node().map(str::to_string),
        iteration: err.iteration(),
        base_message: err.base_message(),
        subs: err.sub_errors().iter().map(capture_sub_error).collect(),
    }
}

// Build the exception instance for one prior-attempt error.  A sub-error
// carrying a server result code gets the mapped ServerError subclass with
// its own detail; anything else gets its client-side type with the retry
// attributes set on the instance.  Sub-errors never nest further.
fn materialize_sub_error(py: Python<'_>, sub: &SubErrorData) -> Py<PyAny> {
    if let Some(rc) = sub.result_code {
        let exc_cls = resolve_server_error_class(py, rc)
            .unwrap_or_else(|_| py.get_type::<ServerError>().into_any());
        let detail = sub.detail.as_ref();
        let sub_code = detail.map(|d| d.sub_code);
        let server_message = detail.map(|d| d.message.clone());
        let exp_trace = detail.and_then(|d| d.exp_trace.as_ref().map(ExpressionTrace::from_core));
        match exc_cls.call1((
            sub.message.clone(),
            ResultCode::from(rc),
            sub.in_doubt,
            sub_code,
            server_message,
            exp_trace,
            sub.node.clone(),
            sub.iteration,
            None::<String>,
            None::<Py<PyAny>>,
        )) {
            Ok(obj) => obj.unbind(),
            Err(e) => e.into_value(py).into_any(),
        }
    } else {
        match sub.kind.type_object(py).call1((sub.message.clone(),)) {
            Ok(obj) => {
                let _ = obj.setattr("result_code", ResultCode::from_i32(sub.code));
                if sub.in_doubt {
                    let _ = obj.setattr("in_doubt", true);
                }
                if let Some(node) = &sub.node {
                    let _ = obj.setattr("node", node);
                }
                if let Some(iteration) = sub.iteration {
                    let _ = obj.setattr("iteration", iteration);
                }
                obj.unbind()
            }
            Err(e) => e.into_value(py).into_any(),
        }
    }
}

fn materialize_sub_exceptions(
    py: Python<'_>,
    subs: &[SubErrorData],
) -> Option<Py<PyAny>> {
    if subs.is_empty() {
        return None;
    }
    let items: Vec<Py<PyAny>> = subs.iter().map(|s| materialize_sub_error(py, s)).collect();
    pyo3::types::PyList::new(py, items)
        .ok()
        .map(|list| list.into_any().unbind())
}

impl PyErrArguments for ServerErrorArgs {
    // Runs when the PyErr is first materialized — for async completions that is
    // on the event-loop/drainer thread inside `set_exception`, never on a Tokio
    // worker.  Builds the concrete subclass *instance* (with any extended error
    // detail) and hands it back as the exception value.
    fn arguments(self, py: Python<'_>) -> Py<PyAny> {
        let exc_cls = resolve_server_error_class(py, self.result_code)
            .unwrap_or_else(|_| py.get_type::<ServerError>().into_any());
        let rc = ResultCode::from(self.result_code);
        let detail = self.detail.as_ref();
        let sub_code = detail.map(|d| d.sub_code);
        let server_message = detail.map(|d| d.message.clone());
        let exp_trace = detail.and_then(|d| d.exp_trace.as_ref().map(ExpressionTrace::from_core));
        let sub_exceptions = materialize_sub_exceptions(py, &self.ctx.subs);
        match exc_cls.call1((
            self.message,
            rc,
            self.in_doubt,
            sub_code,
            server_message,
            exp_trace,
            self.ctx.node,
            self.ctx.iteration,
            Some(self.ctx.base_message),
            sub_exceptions,
        )) {
            Ok(obj) => obj.unbind(),
            // Construction failed — surface that failure's own exception value.
            Err(e) => e.into_value(py).into_any(),
        }
    }
}

// Helper function to create ServerError (or subclass) as a PyErr.
//
// The conversion is *lazy*: `PyErr::new` stores `ServerErrorArgs` and only
// materializes the exception when it is first inspected.  For async completions
// that happens on the event-loop/drainer thread (`set_exception`), NOT on the
// Tokio worker that produced the error.  This keeps `Python::attach` off Tokio
// workers (see the invariant in waker.rs): a worker that attaches registers a
// PyThreadState whose free-threaded finalization teardown
// (`_Py_brc_remove_thread`) segfaults, and attaching mid-finalization panics.
//
// The subclass instance is built in `arguments()` and passed as the exception
// *value* under the base `ServerError` type; CPython's exception normalization
// narrows the reported type to that subclass, so `except RecordNotFound` (etc.)
// still matches.
fn create_server_error(
    message: String,
    result_code: CoreResultCode,
    in_doubt: bool,
    detail: Option<&aerospike_core::ServerErrorDetail>,
    ctx: RetryContext,
) -> PyErr {
    PyErr::new::<ServerError, _>(ServerErrorArgs {
        message,
        result_code,
        in_doubt,
        detail: detail.cloned(),
        ctx,
    })
}

// Deferred arguments for a client-side error: its result code plus the
// retry/diagnostic context (in-doubt, node, iteration, prior-attempt
// errors).  Lazy for the same reason as `ServerErrorArgs` above:
// `arguments()` runs when the PyErr is first materialized, on the
// event-loop/drainer thread, never on a Tokio worker.  Builds the exception
// instance and sets the attributes on it, overriding the class defaults that
// `AerospikeError` declares on the Python side.
struct RetryCtxArgs<T> {
    message: String,
    result_code: i32,
    in_doubt: bool,
    ctx: RetryContext,
    // `fn() -> T` keeps this Send + Sync regardless of T: the generated
    // exception types wrap `PyAny`, which is not Sync.
    _cls: PhantomData<fn() -> T>,
}

impl<T: PyTypeInfo + 'static> PyErrArguments for RetryCtxArgs<T> {
    fn arguments(self, py: Python<'_>) -> Py<PyAny> {
        match py.get_type::<T>().call1((self.message,)) {
            Ok(obj) => {
                // Best effort: a failed setattr falls back to the class
                // default rather than masking the original error.
                let _ = obj.setattr("result_code", ResultCode::from_i32(self.result_code));
                if self.in_doubt {
                    let _ = obj.setattr("in_doubt", true);
                }
                if let Some(node) = &self.ctx.node {
                    let _ = obj.setattr("node", node);
                }
                if let Some(iteration) = self.ctx.iteration {
                    let _ = obj.setattr("iteration", iteration);
                }
                let _ = obj.setattr("base_message", &self.ctx.base_message);
                if let Some(subs) = materialize_sub_exceptions(py, &self.ctx.subs) {
                    let _ = obj.setattr("sub_exceptions", subs);
                }
                obj.unbind()
            }
            // Construction failed — surface that failure's own exception value.
            Err(e) => e.into_value(py).into_any(),
        }
    }
}

// Deferred arguments for a batch-wide failure. Same lazy contract as
// `RetryCtxArgs` (materialized on the event-loop/drainer thread, never on a
// Tokio worker), plus the per-key `BatchRecord` outcomes so callers can report
// truthful per-row results.
struct BatchFailedArgs {
    message: String,
    result_code: i32,
    in_doubt: bool,
    ctx: RetryContext,
    records: Vec<aerospike_core::BatchRecord>,
}

impl PyErrArguments for BatchFailedArgs {
    fn arguments(self, py: Python<'_>) -> Py<PyAny> {
        match py.get_type::<BatchFailedError>().call1((self.message,)) {
            Ok(obj) => {
                let _ = obj.setattr("result_code", ResultCode::from_i32(self.result_code));
                if self.in_doubt {
                    let _ = obj.setattr("in_doubt", true);
                }
                if let Some(node) = &self.ctx.node {
                    let _ = obj.setattr("node", node);
                }
                if let Some(iteration) = self.ctx.iteration {
                    let _ = obj.setattr("iteration", iteration);
                }
                let _ = obj.setattr("base_message", &self.ctx.base_message);
                if let Some(subs) = materialize_sub_exceptions(py, &self.ctx.subs) {
                    let _ = obj.setattr("sub_exceptions", subs);
                }
                let rows: Vec<Py<PyAny>> = self
                    .records
                    .into_iter()
                    .filter_map(|r| {
                        Py::new(py, crate::policies::BatchRecord::new(r))
                            .ok()
                            .map(pyo3::Py::into_any)
                    })
                    .collect();
                if let Ok(list) = pyo3::types::PyList::new(py, rows) {
                    let _ = obj.setattr("records", list);
                }
                obj.unbind()
            }
            Err(e) => e.into_value(py).into_any(),
        }
    }
}

// Commit failures carry what the other clients expose: which stage failed,
// and the per-key verify/roll outcomes so a caller can do selective recovery.
struct CommitFailedArgs {
    message: String,
    // The commit failure's own code (TXN_FAILED), reported only when no
    // server code tripped the stage.
    result_code: i32,
    in_doubt: bool,
    ctx: RetryContext,
    error_type: aerospike_core::txn::CommitErrorType,
    verify_records: Vec<aerospike_core::BatchRecord>,
    roll_records: Vec<aerospike_core::BatchRecord>,
    // The code that tripped verify or roll, when it came from the server.
    // Reporting the commit failure ahead of the cause chain would otherwise
    // drop it, and it is what retry logic classifies on.
    cause_result_code: Option<CoreResultCode>,
}

impl PyErrArguments for CommitFailedArgs {
    fn arguments(self, py: Python<'_>) -> Py<PyAny> {
        match py.get_type::<CommitFailedError>().call1((self.message,)) {
            Ok(obj) => {
                if self.in_doubt {
                    let _ = obj.setattr("in_doubt", true);
                }
                if let Some(node) = &self.ctx.node {
                    let _ = obj.setattr("node", node);
                }
                if let Some(iteration) = self.ctx.iteration {
                    let _ = obj.setattr("iteration", iteration);
                }
                let _ = obj.setattr("base_message", &self.ctx.base_message);
                if let Some(subs) = materialize_sub_exceptions(py, &self.ctx.subs) {
                    let _ = obj.setattr("sub_exceptions", subs);
                }
                let rc = match self.cause_result_code {
                    Some(rc) => ResultCode::from(rc),
                    None => ResultCode::from_i32(self.result_code),
                };
                let _ = obj.setattr("result_code", rc);
                let _ = obj.setattr(
                    "commit_error_type",
                    crate::enums::CommitErrorType::from(self.error_type),
                );
                for (attr, records) in [
                    ("verify_records", self.verify_records),
                    ("roll_records", self.roll_records),
                ] {
                    let rows: Vec<Py<PyAny>> = records
                        .into_iter()
                        .filter_map(|r| {
                            Py::new(py, crate::policies::BatchRecord::new(r))
                                .ok()
                                .map(pyo3::Py::into_any)
                        })
                        .collect();
                    if let Ok(list) = pyo3::types::PyList::new(py, rows) {
                        let _ = obj.setattr(attr, list);
                    }
                }
                obj.unbind()
            }
            Err(e) => e.into_value(py).into_any(),
        }
    }
}

// Every client-side failure carries its result code, so every one takes the
// lazy path: the instance is built, and the code and any retry context set
// on it, only when the exception is first materialized.  Nothing here runs
// on a Tokio worker, and no failure is raised on a hot path — a client error
// means the command is not going to complete.
fn client_err<T>(message: String, result_code: i32, in_doubt: bool, ctx: RetryContext) -> PyErr
where
    T: PyTypeInfo + 'static,
{
    PyErr::new::<T, _>(RetryCtxArgs::<T> {
        message,
        result_code,
        in_doubt,
        ctx,
        _cls: PhantomData,
    })
}

/// Convert a caught Rust panic into a lazy ``RuntimeError`` carrying the panic
/// message. Lazy, so it is safe to build on a Tokio worker or with the GIL
/// released.
pub(crate) fn panic_to_pyerr(panic: Box<dyn std::any::Any + Send>) -> PyErr {
    let msg = if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = panic.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Rust panic".to_string()
    };
    pyo3::exceptions::PyRuntimeError::new_err(msg)
}

create_exception!(aerospike_native.exceptions, UDFBadResponse, AerospikeError);
create_exception!(aerospike_native.exceptions, TimeoutError, AerospikeError);
create_exception!(aerospike_native.exceptions, BadResponse, AerospikeError);

// Connection-related exceptions
create_exception!(aerospike_native.exceptions, ConnectionError, AerospikeError);
create_exception!(aerospike_native.exceptions, InvalidNodeError, AerospikeError);
create_exception!(aerospike_native.exceptions, InvalidNamespaceError, AerospikeError);
create_exception!(aerospike_native.exceptions, NoMoreConnections, AerospikeError);
create_exception!(aerospike_native.exceptions, RecvError, AerospikeError);

// Data parsing/validation exceptions
create_exception!(aerospike_native.exceptions, Base64DecodeError, AerospikeError);
create_exception!(aerospike_native.exceptions, InvalidUTF8, AerospikeError);
create_exception!(aerospike_native.exceptions, ParseAddressError, AerospikeError);
create_exception!(aerospike_native.exceptions, ParseIntError, AerospikeError);
create_exception!(aerospike_native.exceptions, ValueError, AerospikeError);

// System/IO exceptions
create_exception!(aerospike_native.exceptions, IoError, AerospikeError);
create_exception!(aerospike_native.exceptions, PasswordHashError, AerospikeError);

// Client configuration exceptions
create_exception!(aerospike_native.exceptions, InvalidRustClientArgs, AerospikeError);

// Client-side errors
create_exception!(aerospike_native.exceptions, ClientError, AerospikeError);
create_exception!(aerospike_native.exceptions, CommitFailedError, AerospikeError);

// A batch command failed as a whole. Subclasses ClientError so existing
// `except ClientError` handlers keep matching. Carries `records`: the
// per-key `BatchRecord` outcomes core attached to the failure — rows the
// server answered keep their result, unanswered rows carry the stamped
// result code (TIMEOUT on client timeouts) and per-row in-doubt flag.
create_exception!(aerospike_native.exceptions, BatchFailedError, ClientError);

// Per-node circuit breaker tripped (client-side, not sent to server). Carries
// the offending node identifier in the exception message. Raised when a node
// exceeds the policy's `max_error_rate` over `error_rate_window` ticks, so the
// client backs off rather than forwarding more commands to that node.
create_exception!(aerospike_native.exceptions, MaxErrorRate, AerospikeError);


// Must define a wrapper type because of the orphan rule
pub struct RustClientError(pub(crate) Error);

/// Raise a whole-batch failure with the per-key outcomes attached.
///
/// Core returns only the error that ended the batch; the rows stay on the
/// caller's operations. This builds the exception a wrapping error would
/// have produced -- `BATCH_FAILED` over `source`, inheriting its in-doubt
/// flag and node -- and carries the rows on it. A server result code
/// anywhere in `source` still wins and raises the mapped `ServerError`.
pub(crate) fn batch_failed_error(
    records: Vec<aerospike_core::BatchRecord>,
    source: Error,
) -> PyErr {
    let rc = i32::from(aerospike_core::ClientResultCode::BatchFailed);
    let in_doubt = source.in_doubt();
    let base_message = format!("Batch failed ({} records)", records.len());
    let doubt = if in_doubt { ", In Doubt: true" } else { "" };
    let message = format!("Error {rc}{doubt}: {base_message}\ncaused by: {source}");
    let ctx = RetryContext {
        node: source.node().map(str::to_string),
        iteration: None,
        base_message,
        subs: Vec::new(),
    };
    if let Some(server_rc) = source.server_result_code() {
        let detail = source.server_error_detail();
        return create_server_error(message, server_rc, in_doubt, detail, ctx);
    }
    PyErr::new::<BatchFailedError, _>(BatchFailedArgs {
        message,
        result_code: rc,
        in_doubt,
        ctx,
        records,
    })
}

impl From<RustClientError> for PyErr {
    fn from(value: RustClientError) -> Self {
        // RustClientError -> Error -> Custom Exception Classes.
        //
        // The core `Error` is now an opaque struct carrying an `ErrorKind`
        // plus metadata that drills through the retry cause chain (server
        // result code, in-doubt, node, extended detail). Retry wrapping is
        // handled entirely by core, so we no longer walk a `Chain` variant
        // by hand: the accessors already surface the most specific reachable
        // server code, and `Display` renders the full context (iteration,
        // node, sub-errors, cause) for the message.
        let err = value.0;

        // A commit failure reports itself, ahead of the cause-chain rule below.
        // Core wraps the error that tripped verify or roll as the cause, and
        // that cause carries a server result code -- so deferring to the chain
        // would report a bare version mismatch and drop the two things only
        // this wrapper has: which stage failed, and the per-key outcomes that
        // say whether anything landed. The triggering code stays reachable on
        // `__cause__` and in the message.
        if let ErrorKind::Commit {
            error_type,
            verify_records,
            roll_records,
        } = err.kind()
        {
            let in_doubt = err.in_doubt();
            let ctx = capture_retry_context(&err);
            let cause_result_code = err.server_result_code();
            let message = match cause_result_code {
                Some(rc) => format!("{error_type} (in_doubt={in_doubt}, cause: {rc})"),
                None => format!("{error_type} (in_doubt={in_doubt})"),
            };
            return PyErr::new::<CommitFailedError, _>(CommitFailedArgs {
                message,
                result_code: err.result_code(),
                in_doubt,
                ctx,
                error_type: error_type.clone(),
                verify_records: verify_records.clone(),
                roll_records: roll_records.clone(),
                cause_result_code,
            });
        }

        // A server result code reachable anywhere in the cause chain wins: it
        // carries the typed ResultCode + extended detail + in-doubt + node,
        // and maps to the concrete ServerError subclass (RecordNotFound, etc.).
        if let Some(result_code) = err.server_result_code() {
            let in_doubt = err.in_doubt();
            let detail = err.server_error_detail();
            // Core's Display is the message: `Error <code>[, SubCode: N]
            // [, iter=N][, In Doubt: true][, node=X]: <base message>`, the
            // base message carrying any extended detail. Client-side errors
            // and prior-attempt sub-errors already render the same way.
            let message = err.to_string();
            let ctx = capture_retry_context(&err);
            return create_server_error(message, result_code, in_doubt, detail, ctx);
        }

        // Otherwise dispatch on the specific client-side failure. `Display`
        // carries the full retry context, so `except TimeoutError` /
        // `except ConnectionError` still match retried failures.
        let msg = err.to_string();
        // Typed in-doubt from core; `in_doubt()` walks the cause chain, so a
        // wrapper (retry decoration, NoMoreConnections over an in-doubt
        // failure) inherits it.  Every kind that can carry a cause
        // chain routes through `client_err`; the pure conversion failures
        // (Base64 .. PwHash) never do and stay on `new_err`.
        let in_doubt = err.in_doubt();
        // Core's flat code: negative for a client-assigned failure, or the
        // shared TIMEOUT code for a client-side deadline.
        let rc = err.result_code();
        let ctx = capture_retry_context(&err);
        match err.kind() {
            ErrorKind::Timeout => client_err::<TimeoutError>(msg, rc, in_doubt, ctx),
            ErrorKind::Connection | ErrorKind::ConnectionPoolEmpty => {
                client_err::<ConnectionError>(msg, rc, in_doubt, ctx)
            }
            ErrorKind::NoMoreConnections => client_err::<NoMoreConnections>(msg, rc, in_doubt, ctx),
            ErrorKind::MaxErrorRate => client_err::<MaxErrorRate>(msg, rc, in_doubt, ctx),
            ErrorKind::InvalidNode => client_err::<InvalidNodeError>(msg, rc, in_doubt, ctx),
            ErrorKind::InvalidNamespace => {
                client_err::<InvalidNamespaceError>(msg, rc, in_doubt, ctx)
            }
            ErrorKind::InvalidArgument => client_err::<ValueError>(msg, rc, in_doubt, ctx),
            ErrorKind::BadResponse | ErrorKind::ParsePeers => {
                client_err::<BadResponse>(msg, rc, in_doubt, ctx)
            }
            ErrorKind::UdfBadResponse => client_err::<UDFBadResponse>(msg, rc, in_doubt, ctx),
            // The conversion failures keep their inner message; they carry
            // core's PARSE_ERROR / CLIENT_ERROR / SERIALIZE_ERROR code.
            ErrorKind::InvalidUtf8(e) => client_err::<InvalidUTF8>(e.to_string(), rc, in_doubt, ctx),
            ErrorKind::Io(e) => client_err::<IoError>(e.to_string(), rc, in_doubt, ctx),
            ErrorKind::ParseAddr(e) => {
                client_err::<ParseAddressError>(e.to_string(), rc, in_doubt, ctx)
            }
            ErrorKind::ParseInt(e) => client_err::<ParseIntError>(e.to_string(), rc, in_doubt, ctx),
            // Client / StreamTerminated / BatchRow / Async and
            // any future kinds fall back to the generic client error, keeping
            // the full context from `Display`. (Server / Timeout / Connection
            // etc. are handled above.)
            _ => client_err::<ClientError>(msg, rc, in_doubt, ctx),
        }
    }
}
