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

//! Single persistent waker thread for `call_soon_threadsafe` dispatch.
//!
//! Tokio workers must not call `Python::attach` per completion: each
//! `PyGILState_Release` runs `PyThreadState_Clear`, which mimalloc's
//! abandoned-segment collector picks up at ~9.66% process CPU under
//! sustained async load.  Pinning each Tokio worker's thread-state
//! (Branch D) hangs at `z=32` even with the uvloop FT gate active
//! (zero-byte stdout wedge; confirmed 2026-06-18) — the failure mode
//! is intrinsic to keeping many concurrent persistent thread-states
//! on Tokio workers, not a uvloop side effect.
//!
//! Instead: Tokio workers push pure-Rust wake tokens (`Arc<CompletionInner>`)
//! over an mpsc channel.  ONE persistent daemon thread, spawned via Python's
//! `threading.Thread` so CPython owns its thread-state lifecycle, consumes
//! tokens and issues `call_soon_threadsafe(drainer)` to the correct asyncio
//! loop.  Because exactly one extra thread is persistently attached (not
//! many Tokio workers), the failure condition Branch D hit does not apply.
//!
//! Per-batch semantics are unchanged: `CompletionInner::drain_scheduled`
//! still gates wake tokens to one per batch, and `drain()` still runs on the
//! loop thread.

use std::sync::atomic::Ordering;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::completion::CompletionInner;

/// Bounded so a runaway producer can't grow the channel without limit.  Each
/// token gates to one-per-batch, so even at 1M ops/sec the channel sees at
/// most a few thousand tokens/sec — 65k is comfortable headroom.
const WAKER_CHANNEL_CAPACITY: usize = 65_536;

static SENDER: OnceLock<SyncSender<Arc<CompletionInner>>> = OnceLock::new();
// The Receiver is moved into the spawned Python thread on first start.
// Stashed in a global Mutex<Option<...>> so the Python-side recv loop can
// `take()` it; `OnceLock` alone wouldn't allow the take().
static PENDING_RX: OnceLock<Mutex<Option<Receiver<Arc<CompletionInner>>>>> = OnceLock::new();

/// Tokio-worker-side: push a wake token.  Pure Rust, no GIL.
///
/// Returns `Err` on channel overflow OR if the waker hasn't been initialized
/// yet (extremely unlikely — `ensure_waker` runs from `CompletionBridge::new`).
/// Callers MUST reset `inner.drain_scheduled` to false on `Err`, otherwise
/// the queue stalls — the gate stays armed but no wake will fire, and the
/// next op's enqueue won't try to re-arm.
#[inline]
#[must_use = "channel overflow requires resetting drain_scheduled to avoid a stall"]
pub(crate) fn send_wake(inner: Arc<CompletionInner>) -> Result<(), ()> {
    match SENDER.get() {
        Some(tx) => tx.try_send(inner).map_err(|_| ()),
        None => Err(()),
    }
}

/// Lazy-initialize the single waker thread.  Idempotent across all
/// `CompletionBridge::new` calls (AsyncPool may construct many).
///
/// Must be called under GIL (uses Python to spawn a `threading.Thread`).
pub(crate) fn ensure_waker(py: Python<'_>) -> PyResult<()> {
    if SENDER.get().is_some() {
        return Ok(()); // already initialized
    }
    let (tx, rx) = sync_channel(WAKER_CHANNEL_CAPACITY);
    // Race: another thread may have raced us to set SENDER.  set() returns
    // Err if already set; in that case our channel is dropped and we bail.
    if SENDER.set(tx).is_err() {
        return Ok(());
    }
    // Store the receiver so the spawned Python thread can take() it.
    let _ = PENDING_RX.set(Mutex::new(Some(rx)));

    // Spawn the daemon Python thread.  Using Python's `threading.Thread`
    // (Variant 1 of the plan) keeps the persistent PyThreadState under
    // CPython's lifecycle management — no unsafe `PyGILState_Ensure`
    // without a matching Release as Branch D required.
    let threading = py.import(pyo3::intern!(py, "threading"))?;
    let kwargs = PyDict::new(py);
    let target = wrap_pyfunction!(waker_loop_py, py)?;
    kwargs.set_item(pyo3::intern!(py, "target"), target)?;
    kwargs.set_item(pyo3::intern!(py, "name"), "aerospike-waker")?;
    kwargs.set_item(pyo3::intern!(py, "daemon"), true)?;
    let thread = threading.call_method(pyo3::intern!(py, "Thread"), (), Some(&kwargs))?;
    thread.call_method0(pyo3::intern!(py, "start"))?;
    Ok(())
}

/// The waker thread's recv loop.  Runs as a Python daemon thread; CPython
/// owns this thread's `PyThreadState` for the interpreter's lifetime.
#[pyfunction]
fn waker_loop_py(py: Python<'_>) {
    // Take the receiver from the global; if already taken (started twice),
    // bail.  This is defensive — `ensure_waker` is idempotent and only
    // starts the thread once via `SENDER.set` racing semantics.
    let rx = match PENDING_RX.get().and_then(|m| m.lock().ok()).and_then(|mut g| g.take()) {
        Some(rx) => rx,
        None => return,
    };

    loop {
        // Release the GIL while blocked on the channel.  Required for GIL-on
        // builds (otherwise we'd block other Python threads waiting to run);
        // benign for FT (no GIL exists to release).
        //
        // `Python::detach` won't compile here because `&Receiver` isn't
        // `Send` (Receiver is !Sync) — go straight to the FFI.  Same
        // mechanism PyO3's `detach` uses internally; we just sidestep its
        // closure-Send bound.
        //
        // A pipe-transport wake is one write to a kernel fd, so it is sent
        // from inside the detached section. Re-attaching for it would make
        // every wake wait for the GIL, and a loop thread that is busy without
        // ever blocking can hold that wait for many milliseconds. Only a
        // wake that has to touch Python leaves the detached section.
        let token = unsafe {
            let saved = pyo3::ffi::PyEval_SaveThread();
            let result = loop {
                match rx.recv() {
                    Ok(inner) => {
                        #[cfg(unix)]
                        {
                            match crate::completion::pipe_wake(&inner) {
                                Some(Ok(())) => continue,
                                Some(Err(_)) => break Ok((inner, true)),
                                None => {}
                            }
                        }
                        break Ok((inner, false));
                    }
                    Err(e) => break Err(e),
                }
            };
            pyo3::ffi::PyEval_RestoreThread(saved);
            result
        };
        let (inner, pipe_broken) = match token {
            Ok(t) => t,
            // All senders dropped — interpreter shutdown.  Exit cleanly.
            Err(_) => return,
        };

        // Finalization guard: if the interpreter is not initialized or is
        // being torn down, touching Python would crash (pyo3 0.29 rejects
        // attach during finalization; on FT builds the thread-state teardown
        // null-derefs).
        if crate::completion::interpreter_unavailable() {
            return;
        }

        if pipe_broken {
            // Write end broke (loop's read end closed): latch + fail so
            // callers don't hang, mirroring the call_soon-failure path.
            inner.closed.store(true, Ordering::Release);
            inner.fail_all_pending();
            continue;
        }

        // We're back attached after detach(); call_soon_threadsafe runs
        // here on our persistent PyThreadState — no churn per token.
        let result: PyResult<()> = (|| {
            let el = inner.owning_loop.bind(py);
            let drainer = inner
                .drainer
                .get()
                .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err(
                    "drainer not set on CompletionInner",
                ))?;
            el.call_method1(
                pyo3::intern!(py, "call_soon_threadsafe"),
                (drainer.bind(py),),
            )?;
            Ok(())
        })();

        if result.is_err() {
            // Loop closed during/after enqueue (call_soon_threadsafe rejects
            // submissions on a closed loop).  Latch + drain pending so
            // callers don't hang.
            inner.closed.store(true, Ordering::Release);
            inner.fail_all_pending();
        }
    }
}
