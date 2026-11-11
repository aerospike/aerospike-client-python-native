// Copyright 2025-2026 Aerospike, Inc.
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

//! Per-`Client` batched completion bridge for async Python futures.
//!
//! Each `Client` owns one bridge → one queue → one drainer, all paired to the
//! event loop that created the client.  Under a multi-loop pool that means
//! completions never cross loops: loop A's completions enqueue into client-A's
//! bridge and wake client-A's drainer via `call_soon_threadsafe`.
//!
//! Inside one bridge, completions still batch — 32-way concurrency collapses
//! 32 GIL acquisitions per completion burst into 1.
//!
//! The bridge captures its owning event loop at construction and asserts it on
//! every call.  Sharing one `Client` across two event loops fails loud at the
//! first cross-loop call instead of silently routing a future from loop B
//! through loop A's drainer — a violation that is masked by the GIL but real
//! under free-threading.

use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use futures::FutureExt;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;

use pyo3_async_runtimes::tokio as pyo3_asyncio;

#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

use crate::client_runtime::ClientRuntime;
use crate::waker;

type Converter = Box<dyn FnOnce(Python<'_>) -> PyResult<Py<PyAny>> + Send>;

/// Cross-thread wake transport. Instead of the waker thread calling
/// `loop.call_soon_threadsafe`, the waker writes one byte to `tx`; a
/// `loop.add_reader(rx_fd, PipeDrainer)` callback then runs `drain()` on the
/// loop thread. The foreign thread touches only a kernel fd, which buys two
/// things: it needs no GIL, so a busy loop thread cannot hold a wake back,
/// and under uvloop on a free-threaded build it stays off
/// `call_soon_threadsafe`, whose ready-queue mutation trips uvloop's libuv
/// free-threading race (MagicStack/uvloop #720, unfixed in release 0.22.1).
/// Unix only (see `should_use_pipe`).
#[cfg(unix)]
struct PipeTransport {
    /// Waker-thread write end. One byte per completion burst; coalesced by the
    /// reader draining to `WouldBlock`.
    tx: UnixStream,
    /// Loop-thread read end, watched via `add_reader`.
    rx: UnixStream,
}

struct PendingResult {
    future: Py<PyAny>,
    result: Result<Converter, PyErr>,
}

/// State shared between a bridge and its drainer.
pub(crate) struct CompletionInner {
    queue: Mutex<Vec<PendingResult>>,
    drain_scheduled: AtomicBool,
    /// Latched true on first `call_soon_threadsafe` failure (loop closed).
    /// Once set, subsequent enqueues fast-path-fail the pending Python future
    /// rather than queuing it — prevents indefinite hangs on shutdown races.
    pub(crate) closed: AtomicBool,
    /// Event loop captured at bridge construction.  Every operation routed
    /// through this bridge must run on this loop; mismatches fail at the
    /// `batched_future_into_py` entry rather than silently corrupting state
    /// (`set_result` on loop B's future from loop A's thread).
    pub(crate) owning_loop: Py<PyAny>,
    /// Drainer pyobject for this bridge.  Populated by `CompletionBridge::new`
    /// after the Arc<Inner> exists (the Drainer holds an `Arc<CompletionInner>`
    /// back-reference, so it can't be constructed before Inner).  Reads on the
    /// hot path go through the waker thread.
    pub(crate) drainer: OnceLock<Py<CompletionDrainer>>,
    /// Per-Client runtime: when Some, spawn each future on this dedicated
    /// Tokio runtime instead of the shared global multi-thread one. Set
    /// from `ClientPolicy.per_client_runtime_workers`. Held by Arc so the
    /// runtime outlives any in-flight futures.
    client_rt: Option<Arc<ClientRuntime>>,
    /// Pipe-wake transport. Set when `should_use_pipe` selects it and the
    /// loop accepted the reader; empty uses the `call_soon_threadsafe` path.
    /// Holds both socketpair ends so they close when this `CompletionInner`
    /// drops.
    ///
    /// Lifetime: the loop-side reader (`add_reader`) holds a `Py<PipeDrainer>`
    /// which holds an `Arc<CompletionInner>`, so the transport lives until the
    /// owning loop closes and asyncio drops its readers — the same lifecycle as
    /// the `call_soon_threadsafe` path, and required so a *closed but not yet
    /// dropped* client (e.g. `is_connected()` after `close()`) still delivers
    /// completions through the reader. A client closed on a still-running loop
    /// therefore keeps its two socketpair fds until loop close; negligible for
    /// pooled/typical usage (client lifetime ≈ loop lifetime), and only worth
    /// revisiting for high-churn client creation on one long-lived loop.
    #[cfg(unix)]
    pipe: OnceLock<PipeTransport>,
}

/// Owned by each `Client`.  Thin wrapper around `Arc<CompletionInner>` so the
/// hot path's per-op clone is a pure `Arc::clone` — no GIL acquisition, no
/// `Py<>` refcount bump.
#[derive(Clone)]
pub(crate) struct CompletionBridge {
    inner: Arc<CompletionInner>,
}

/// Python-callable invoked on the event loop thread via `call_soon_threadsafe`.
/// Holds an Arc to its bridge's inner state so it drains the right queue.
#[pyclass]
pub(crate) struct CompletionDrainer {
    inner: Arc<CompletionInner>,
}

#[pymethods]
impl CompletionDrainer {
    fn __call__(&self, py: Python<'_>) -> PyResult<()> {
        drain(py, &self.inner);
        Ok(())
    }
}

/// Loop-thread reader for the pipe-wake transport. Registered via
/// `loop.add_reader(rx_fd, PipeDrainer)`; asyncio invokes it (0-arg) whenever
/// the read end is readable. Drains the coalesced wake bytes, then runs the
/// same `drain()` the `CompletionDrainer` would — semantics are identical, only
/// the wake transport differs.
#[cfg(unix)]
#[pyclass]
pub(crate) struct PipeDrainer {
    inner: Arc<CompletionInner>,
}

#[cfg(unix)]
#[pymethods]
impl PipeDrainer {
    fn __call__(&self, py: Python<'_>) -> PyResult<()> {
        if let Some(p) = self.inner.pipe.get() {
            drain_fd(&p.rx);
        }
        drain(py, &self.inner);
        Ok(())
    }
}

/// Waker-thread side: write one wake byte for a pipe bridge. Returns `None`
/// when the bridge uses the `call_soon_threadsafe` path (caller falls through
/// to it). `WouldBlock` is mapped to `Ok` — a full pipe means a wake is already
/// pending and undrained, which the one-per-burst gate makes benign.
#[cfg(unix)]
pub(crate) fn pipe_wake(inner: &CompletionInner) -> Option<std::io::Result<()>> {
    inner.pipe.get().map(|p| {
        let mut writer: &UnixStream = &p.tx;
        match writer.write(b"x") {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
            Err(e) => Err(e),
        }
    })
}

/// Drain the read end to `WouldBlock`, consuming all coalesced wake bytes so the
/// reader doesn't re-fire for wakes already covered by this `drain()`.
#[cfg(unix)]
fn drain_fd(rx: &UnixStream) {
    let mut buf = [0u8; 256];
    let mut reader: &UnixStream = rx;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break, // writer closed
            Ok(_) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break, // WouldBlock (drained) or hard error — stop either way
        }
    }
}

/// True on a free-threaded (GIL-disabled) build. uvloop's #720 race only
/// manifests without the GIL serializing the cross-thread ready-queue mutation.
#[cfg(unix)]
fn is_free_threaded() -> bool {
    cfg!(Py_GIL_DISABLED)
}

/// Whether the owning loop is a uvloop loop (its type's module is `uvloop*`).
#[cfg(unix)]
fn loop_is_uvloop(py: Python<'_>, owning_loop: &Py<PyAny>) -> bool {
    (|| -> PyResult<bool> {
        let module: String = owning_loop
            .bind(py)
            .get_type()
            .getattr(pyo3::intern!(py, "__module__"))?
            .extract()?;
        Ok(module.starts_with("uvloop"))
    })()
    .unwrap_or(false)
}

/// How the wake transport was chosen.
#[cfg(unix)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum PipeChoice {
    Off,
    /// Picked by default: a loop that cannot watch the fd falls back to
    /// `call_soon_threadsafe`.
    Auto,
    /// Asked for by name: a loop that cannot watch the fd is an error.
    Forced,
}

/// Transport selection. `AEROSPIKE_PIPE_WAKE` = `0` off / `1` force-on /
/// `auto` (default). Unix-only (Windows has no usable `add_reader` for this
/// and no uvloop).
///
/// `auto` takes the pipe on every GIL build, because there the other path
/// makes the waker thread take the GIL for each wake, and a loop thread that
/// stays busy without blocking (a consumer draining buffered stream items)
/// can keep it waiting for tens of milliseconds. A free-threaded build has
/// no such wait and takes the pipe only for uvloop, whose
/// `call_soon_threadsafe` is racy there.
#[cfg(unix)]
fn should_use_pipe(py: Python<'_>, owning_loop: &Py<PyAny>) -> PipeChoice {
    match std::env::var("AEROSPIKE_PIPE_WAKE").ok().as_deref() {
        Some("1") => PipeChoice::Forced,
        Some("auto") | None => {
            if !is_free_threaded() || loop_is_uvloop(py, owning_loop) {
                PipeChoice::Auto
            } else {
                PipeChoice::Off
            }
        }
        Some(_) => PipeChoice::Off,
    }
}

impl CompletionInner {
    /// Tokio-worker-side entry: queue a result for batch delivery via the
    /// waker thread.  Pure Rust — no `Python::attach`, no GIL acquisition.
    fn enqueue(inner: &Arc<Self>, pr: PendingResult) {
        // Fast-path: bridge already known dead.  Fail the future immediately
        // so the caller doesn't hang.
        if inner.closed.load(Ordering::Acquire) {
            fail_pr(pr, "event loop is closed");
            return;
        }
        inner.queue.lock().unwrap().push(pr);
        if !inner.drain_scheduled.swap(true, Ordering::AcqRel) {
            // First completion in this batch — hand a wake token to the
            // single persistent waker thread.  No GIL needed here; the waker
            // thread does the `call_soon_threadsafe(drainer)` call with its
            // own persistent PyThreadState.
            if waker::send_wake(inner.clone()).is_err() {
                // Channel overflowed (or waker not initialized).  Reset the
                // gate so the NEXT enqueue can re-arm — otherwise the queue
                // stalls with items pushed but no drain ever fired.
                //
                // Race-benign: a duplicate wake from the resetting enqueue
                // is harmless (drain only fires when the gate is won, and
                // drain itself idempotently resets the gate).
                inner.drain_scheduled.store(false, Ordering::Release);
            }
        }
        // Defensive: the waker thread may have failed call_soon_threadsafe
        // (loop closed) and latched `closed` after our push but before our
        // swap-check.  fail_all_pending is idempotent (Mutex+take — second
        // caller drains empty), so the rare double-call here is harmless.
        if inner.closed.load(Ordering::Acquire) {
            inner.fail_all_pending();
        }
    }

    pub(crate) fn fail_all_pending(&self) {
        let pending: Vec<PendingResult> =
            std::mem::take(&mut *self.queue.lock().unwrap());
        if pending.is_empty() {
            return;
        }
        // Shutdown-safety guard: never `Python::attach` from a Tokio worker
        // while the interpreter is finalizing.  On a free-threaded build,
        // attaching here creates a fresh `PyThreadState` for this worker whose
        // biased-refcount teardown (`_Py_brc_remove_thread`) then null-derefs
        // during finalization (EXC_BAD_ACCESS).  The futures are unreachable
        // during process teardown, so drop `pending` — its `Py` refs release
        // via pyo3's deferred mechanism, no attach required.
        if interpreter_unavailable() {
            return;
        }
        Python::attach(|py| {
            for pr in pending {
                let future = pr.future.bind(py);
                let err =
                    pyo3::exceptions::PyRuntimeError::new_err("event loop is closed");
                let _ =
                    future.call_method1(pyo3::intern!(py, "set_exception"), (err,));
            }
        });
    }
}

impl CompletionBridge {
    pub(crate) fn new(
        py: Python<'_>,
        owning_loop: Py<PyAny>,
        client_rt: Option<Arc<ClientRuntime>>,
    ) -> PyResult<Self> {
        // Ensure the waker thread exists before any completion can fire.
        waker::ensure_waker(py)?;

        #[cfg(unix)]
        let pipe_choice = should_use_pipe(py, &owning_loop);

        let inner = Arc::new(CompletionInner {
            queue: Mutex::new(Vec::new()),
            drain_scheduled: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            owning_loop,
            drainer: OnceLock::new(),
            client_rt,
            #[cfg(unix)]
            pipe: OnceLock::new(),
        });
        let drainer = Py::new(py, CompletionDrainer { inner: inner.clone() })?;
        let _ = inner.drainer.set(drainer);

        // Register the loop-side reader for the pipe transport. The loop is
        // running here (bridge construction is awaited), so add_reader takes
        // effect immediately — before the first completion can fire. The
        // transport is installed only once the loop has accepted the reader,
        // so a bridge never wakes through a pipe nobody watches.
        #[cfg(unix)]
        {
            if pipe_choice != PipeChoice::Off {
                let transport = make_socketpair().map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "pipe-wake: socketpair creation failed: {e}"
                    ))
                })?;
                let reader = Py::new(py, PipeDrainer { inner: inner.clone() })?;
                let added = inner.owning_loop.bind(py).call_method1(
                    pyo3::intern!(py, "add_reader"),
                    (transport.rx.as_raw_fd(), reader),
                );
                match added {
                    Ok(_) => {
                        let _ = inner.pipe.set(transport);
                    }
                    // Not every loop can watch an fd (a proactor-style loop
                    // raises NotImplementedError).
                    Err(_) if pipe_choice == PipeChoice::Auto => {}
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(CompletionBridge { inner })
    }

    /// Reject a call made from an event loop other than the bridge's own.
    ///
    /// A diagnostic, not a safety invariant, so it is compiled into debug
    /// builds only (~0.5-1 µs per op saved in release). In a release build
    /// a spawned wait still fails, later: awaiting its future from another
    /// loop raises. A buffered stream step creates no future, so in a
    /// release build it is not checked at all and succeeds from any loop;
    /// the stream's lock keeps that memory-safe.
    #[cfg_attr(not(debug_assertions), allow(unused_variables, clippy::unnecessary_wraps))]
    pub(crate) fn check_owning_loop(&self, py: Python<'_>) -> PyResult<()> {
        #[cfg(debug_assertions)]
        {
            let locals = pyo3_asyncio::get_current_locals(py)?;
            let event_loop = locals.event_loop(py);
            if !event_loop.is(self.inner.owning_loop.bind(py)) {
                return Err(pyo3::exceptions::PyRuntimeError::new_err(
                    "Aerospike Client used from a different event loop than the one that created it. \
                     One Client must be paired with one event loop — construct a separate Client per \
                     loop (AsyncPool does this automatically).",
                ));
            }
        }
        Ok(())
    }

    /// Deliver completions already queued on this bridge, from the loop
    /// thread, without waiting for a wake to reach the drainer.
    ///
    /// For a consumer that is about to yield after a run of steps that never
    /// blocked the loop. Without the pipe transport on a GIL build, the waker
    /// thread cannot get the GIL from such a loop for tens of milliseconds,
    /// and every operation on this client would wait that long. With the
    /// pipe the wake does arrive, but only after the loop's next poll and a
    /// reader callback; delivering here resumes the waiting tasks in the
    /// turn the consumer is giving up.
    pub(crate) fn deliver_queued(&self, py: Python<'_>) {
        // `drain_scheduled` is left alone: a wake already on its way stays
        // the one outstanding wake, so completions that queue up meanwhile
        // do not each send another. Its drain then finds the queue empty.
        deliver(py, &self.inner);
    }

    /// Arc to the shared inner state. Used by per-op-delivery coalesced-submit
    /// entries that resolve each op's own future through this bridge's drainer
    /// as it completes (see [`deliver_one`]).
    pub(crate) fn inner_arc(&self) -> Arc<CompletionInner> {
        self.inner.clone()
    }

    /// Spawn a driver future on this bridge's runtime — the per-Client runtime
    /// when configured, else the shared global one. Mirrors
    /// [`batched_future_into_py`]'s spawn choice so coalesced submits land on
    /// the same reactor as the ops they drive.
    pub(crate) fn spawn<F>(&self, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        match self.inner.client_rt.clone() {
            Some(rt) => {
                rt.handle().spawn(fut);
            }
            None => {
                pyo3_asyncio::get_runtime().spawn(fut);
            }
        }
    }
}

/// Enqueue one already-created Python future's result for batched delivery
/// through a bridge's drainer — the per-op-delivery primitive for coalesced
/// submits. Each op calls this the instant it completes, so callers of fast
/// keys resume without waiting on their batch-mates (no head-of-line), while
/// the drainer still coalesces the `set_result` calls on the loop thread. The
/// `T -> PyObject` conversion runs inside the drainer, off the Tokio worker.
pub(crate) fn deliver_one<T>(
    inner: &Arc<CompletionInner>,
    future: Py<PyAny>,
    result: PyResult<T>,
) where
    T: for<'a> IntoPyObject<'a> + Send + 'static,
{
    let pr = match result {
        Ok(val) => PendingResult {
            future,
            result: Ok(Box::new(move |py| val.into_py_any(py))),
        },
        Err(e) => PendingResult {
            future,
            result: Err(e),
        },
    };
    CompletionInner::enqueue(inner, pr);
}

/// Create a nonblocking `socketpair` for the pipe-wake transport.
#[cfg(unix)]
fn make_socketpair() -> std::io::Result<PipeTransport> {
    let (tx, rx) = UnixStream::pair()?;
    tx.set_nonblocking(true)?;
    rx.set_nonblocking(true)?;
    Ok(PipeTransport { tx, rx })
}

/// True when the interpreter cannot be safely attached to from a non-Python
/// thread — either not yet initialized or already finalizing.  On free-threaded
/// builds, `Python::attach` from a Tokio worker during finalization registers a
/// `PyThreadState` whose biased-refcount teardown crashes; guard every
/// worker-side cold path with this.
pub(crate) fn interpreter_unavailable() -> bool {
    unsafe {
        if pyo3::ffi::Py_IsInitialized() == 0 {
            return true;
        }
        // `Py_IsFinalizing` is Python 3.13+ (pyo3-ffi gates it on `Py_3_13`).
        // The teardown crash it guards against is a free-threaded (3.13t+)
        // hazard, so on older ABIs the initialized check alone suffices.
        #[cfg(Py_3_13)]
        {
            pyo3::ffi::Py_IsFinalizing() != 0
        }
        #[cfg(not(Py_3_13))]
        {
            false
        }
    }
}

fn fail_pr(pr: PendingResult, msg: &'static str) {
    // Shutdown-safety guard (see `fail_all_pending`): don't attach during
    // finalization.  Dropping `pr` releases its `Py` ref via pyo3's deferred
    // drop; the future is unreachable during teardown anyway.
    if interpreter_unavailable() {
        return;
    }
    Python::attach(|py| {
        let future = pr.future.bind(py);
        let err = pyo3::exceptions::PyRuntimeError::new_err(msg);
        let _ = future.call_method1(pyo3::intern!(py, "set_exception"), (err,));
    });
}

fn drain(py: Python<'_>, inner: &Arc<CompletionInner>) {
    // Allow new batches to schedule while we process.
    inner.drain_scheduled.store(false, Ordering::Release);
    deliver(py, inner);
}

fn deliver(py: Python<'_>, inner: &Arc<CompletionInner>) {
    let pending: Vec<PendingResult> =
        std::mem::take(&mut *inner.queue.lock().unwrap());
    for pr in pending {
        let future = pr.future.bind(py);
        // Skip the cancelled() check: set_result/set_exception on a cancelled
        // future raises InvalidStateError, which `let _` discards.  Saves 3
        // Python dispatches per completion on the hot path.
        match pr.result {
            Ok(converter) => match converter(py) {
                Ok(val) => {
                    let _ = future
                        .call_method1(pyo3::intern!(py, "set_result"), (val,));
                }
                Err(e) => {
                    let _ = future
                        .call_method1(pyo3::intern!(py, "set_exception"), (e,));
                }
            },
            Err(e) => {
                let _ =
                    future.call_method1(pyo3::intern!(py, "set_exception"), (e,));
            }
        }
    }
}

/// Batched replacement for `pyo3_asyncio::future_into_py`, scoped to a
/// client's bridge.
///
/// Asserts the call-time event loop matches the bridge's owning loop, then
/// creates an asyncio `Future`, spawns the Rust future on Tokio, and on
/// completion enqueues the result for batch delivery via the owning loop's
/// `call_soon_threadsafe`.
pub(crate) fn batched_future_into_py<'py, F, T>(
    bridge: &CompletionBridge,
    py: Python<'py>,
    fut: F,
) -> PyResult<Bound<'py, PyAny>>
where
    F: Future<Output = PyResult<T>> + Send + 'static,
    T: for<'a> IntoPyObject<'a> + Send + 'static,
{
    bridge.check_owning_loop(py)?;
    let event_loop = bridge.inner.owning_loop.bind(py);
    let py_fut =
        event_loop.call_method0(pyo3::intern!(py, "create_future"))?;

    let future_ref: Py<PyAny> = py_fut.clone().unbind();
    // Pure Arc::clone — no GIL acquisition, no Py<> refcount bump.  The
    // spawned Tokio task only needs `Arc<CompletionInner>` to enqueue;
    // wake-up is delegated to the persistent waker thread.
    let inner = bridge.inner.clone();
    // Capture the per-Client runtime handle before moving `inner` into the
    // async closure. Arc clone is cheap and keeps the runtime alive for the
    // lifetime of the spawn.
    let client_rt = inner.client_rt.clone();

    let spawn_fut = async move {
        let outcome = AssertUnwindSafe(fut).catch_unwind().await;
        // Backstop for the shutdown-race panic: when a user-code exception
        // escapes `asyncio.run()` while Tokio tasks are still airborne, the
        // interpreter begins finalizing.  Even with the waker thread doing
        // the GIL work, `fail_pr` (cold path) still goes through
        // `Python::attach`, which asserts on `Py_IsInitialized` and panics
        // during finalization.  Catching here keeps the panic from leaving
        // a dead Tokio worker; the asyncio future is unreachable in that
        // scenario anyway, so silently dropping `future_ref` and `inner` is
        // fine.  The stderr noise from the panic hook firing before this
        // catches is suppressed separately by the `take_hook`/`set_hook`
        // filter installed in `lib.rs` module init.
        let _: std::result::Result<(), _> = catch_unwind(AssertUnwindSafe(move || {
            let pr = match outcome {
                Ok(Ok(val)) => PendingResult {
                    future: future_ref,
                    result: Ok(Box::new(move |py| val.into_py_any(py))),
                },
                Ok(Err(e)) => PendingResult {
                    future: future_ref,
                    result: Err(e),
                },
                Err(panic) => PendingResult {
                    future: future_ref,
                    result: Err(crate::errors::panic_to_pyerr(panic)),
                },
            };
            CompletionInner::enqueue(&inner, pr);
        }));
    };

    // Spawn on the per-Client runtime when set; otherwise fall back to
    // the global multi-thread runtime (default behavior).
    match client_rt {
        Some(rt) => {
            rt.handle().spawn(spawn_fut);
        }
        None => {
            pyo3_asyncio::get_runtime().spawn(spawn_fut);
        }
    }

    Ok(py_fut)
}
