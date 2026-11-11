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

//! Awaitables for one step of async stream iteration.
//!
//! A stream item that is already buffered needs no I/O, so handing it out
//! through a spawned Tokio task costs three thread handoffs for nothing.
//! `__anext__` instead takes the item on the loop thread and returns a
//! [`ReadyStep`], which resolves without suspending. Only a step that finds
//! the stream empty goes through the `CompletionBridge`.
//!
//! A step that never suspends never lets the event loop run, so a consumer
//! draining a large buffered result would starve every other task. Once a
//! run of such steps reaches [`YIELD_INTERVAL`] steps or [`YIELD_BUDGET`] of
//! loop time, `__anext__` returns a [`YieldFirstStep`], which gives the loop
//! one turn before it takes anything.

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures::Stream;
use pyo3::exceptions::{PyRuntimeError, PyStopIteration};
use pyo3::prelude::*;
use pyo3::{PyTraverseError, PyVisit};
use pyo3_stub_gen::derive::gen_stub_pyfunction;

/// Most buffered items handed out back to back before one step yields to the
/// loop.
pub(crate) const YIELD_INTERVAL: u32 = 64;

/// Longest a run of buffered steps holds the loop before one step yields. The
/// run includes the consumer's own work between steps, so a slow loop body
/// yields after fewer items.
pub(crate) const YIELD_BUDGET: Duration = Duration::from_micros(100);

/// Steps between clock reads, which keeps the clock off most steps.
const BUDGET_CHECK_INTERVAL: u32 = 8;

/// A consumer's current run of steps that did not suspend it.
pub(crate) struct BufferedStreak {
    /// Items handed out in this run.
    steps: u32,
    /// Item count at which the clock is read next.
    next_check: u32,
    started: Option<Instant>,
}

impl BufferedStreak {
    pub(crate) const fn new() -> Self {
        Self { steps: 0, next_check: BUDGET_CHECK_INTERVAL, started: None }
    }

    /// Whether the next step should yield to the loop before taking.
    pub(crate) fn is_due(&mut self) -> bool {
        if self.steps >= YIELD_INTERVAL {
            return true;
        }
        if self.steps < self.next_check {
            return false;
        }
        self.next_check = self.steps + BUDGET_CHECK_INTERVAL;
        self.started.is_some_and(|started| started.elapsed() >= YIELD_BUDGET)
    }

    /// Count a step that handed out one buffered item.
    pub(crate) fn extend(&mut self) {
        self.extend_by(1);
    }

    /// Count a step that handed out `items` buffered items at once.
    pub(crate) fn extend_by(&mut self, items: u32) {
        if self.started.is_none() {
            self.started = Some(Instant::now());
        }
        self.steps = self.steps.saturating_add(items);
    }

    /// End the run: the consumer is about to let the loop take a turn.
    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }
}

/// A stream that can take back the item that ended a batch.
///
/// A batch stops at the first error or at the end of the stream, but it can
/// only learn of either by taking it off the stream. The items gathered so
/// far still have to be returned, so the terminal item is parked here and
/// comes out again on the next poll, whichever way the stream is read next.
pub(crate) struct Parking<S: Stream> {
    inner: S,
    parked: Option<Option<S::Item>>,
}

impl<S: Stream + Unpin> Parking<S> {
    pub(crate) fn new(inner: S) -> Self {
        Self { inner, parked: None }
    }

    /// Hand `item` back, to be the result of the next poll.
    pub(crate) fn park(&mut self, item: Option<S::Item>) {
        self.parked = Some(item);
    }
}

impl<S: Stream + Unpin> Stream for Parking<S>
where
    S::Item: Unpin,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        let this = self.get_mut();
        if let Some(item) = this.parked.take() {
            return Poll::Ready(item);
        }
        Pin::new(&mut this.inner).poll_next(cx)
    }
}

/// Outcome of trying to take the next item without waiting.
pub(crate) enum Taken {
    /// An item, a stream error, or the end of the stream
    /// (`StopAsyncIteration`), ready to hand to the awaiting task.
    Ready(PyResult<Py<PyAny>>),
    /// Nothing is buffered, or another step holds the stream.
    Pending,
}

/// What a [`YieldFirstStep`] needs from the stream it steps.
pub(crate) trait StepSource: Send + Sync {
    /// Take the next item if one is buffered. Runs on the loop thread.
    fn try_take(&self, py: Python<'_>) -> Taken;

    /// Start a wait for the next item and return its asyncio future.
    fn spawn_next(&self, py: Python<'_>) -> PyResult<Py<PyAny>>;
}

/// Poll `stream` once without registering for a wakeup.
///
/// A `Pending` result must be followed by a real poll (the spawned path):
/// that poll installs its own waker, so the wakeup skipped here is not lost.
pub(crate) fn poll_buffered<S>(stream: Pin<&mut S>) -> Poll<Option<S::Item>>
where
    S: Stream + ?Sized,
{
    let mut cx = Context::from_waker(futures::task::noop_waker_ref());
    stream.poll_next(&mut cx)
}

/// Wrap a taken item in an awaitable that resolves without suspending.
pub(crate) fn ready_step(py: Python<'_>, result: PyResult<Py<PyAny>>) -> PyResult<Py<PyAny>> {
    Ok(Py::new(py, ReadyStep { result: Some(result) })?.into_any())
}

/// Build an awaitable that yields to the loop once, then steps `source`.
pub(crate) fn yield_first_step(
    py: Python<'_>,
    source: Box<dyn StepSource>,
) -> PyResult<Py<PyAny>> {
    Ok(Py::new(py, YieldFirstStep { source, phase: Phase::Start })?.into_any())
}

/// Internal: wrap ``value`` in an awaitable that resolves to it without
/// suspending. Not part of the public API; the SDK's query streams use it to
/// hand out a row they already hold without a coroutine per row.
#[pyfunction]
#[gen_stub_pyfunction(module = "_native")]
#[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Any]", imports=("typing")))]
pub(crate) fn _ready_step(py: Python<'_>, value: Py<PyAny>) -> PyResult<Py<PyAny>> {
    ready_step(py, Ok(value))
}

fn finish(result: PyResult<Py<PyAny>>) -> PyResult<Py<PyAny>> {
    // The 1-tuple keeps a tuple item (a batch row) from being unpacked into
    // StopIteration's arguments, which would return only its first element.
    result.and_then(|value| Err(PyStopIteration::new_err((value,))))
}

fn spent() -> PyErr {
    PyRuntimeError::new_err("cannot reuse an already awaited stream step")
}

#[pyclass(name = "_ReadyStep", module = "_native")]
#[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 256))]
pub(crate) struct ReadyStep {
    result: Option<PyResult<Py<PyAny>>>,
}

#[pymethods]
impl ReadyStep {
    fn __await__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Py<PyAny>> {
        finish(self.result.take().ok_or_else(spent)?)
    }
}

enum Phase {
    Start,
    Resumed,
    /// Nothing was buffered after the yield; forwarding the `__await__`
    /// iterator of the spawned wait.
    Waiting(Py<PyAny>),
    Done,
}

#[pyclass(name = "_YieldFirstStep", module = "_native")]
pub(crate) struct YieldFirstStep {
    source: Box<dyn StepSource>,
    phase: Phase,
}

#[pymethods]
impl YieldFirstStep {
    fn __await__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match std::mem::replace(&mut self.phase, Phase::Done) {
            Phase::Start => {
                // Yield before taking: an item taken first would already be
                // behind the stream's resume cursor, and a cancellation
                // landing on this yield would drop it.
                self.phase = Phase::Resumed;
                Ok(py.None())
            }
            Phase::Resumed => match self.source.try_take(py) {
                Taken::Ready(result) => finish(result),
                Taken::Pending => {
                    let steps = self
                        .source
                        .spawn_next(py)?
                        .bind(py)
                        .call_method0(pyo3::intern!(py, "__await__"))?;
                    self.forward(steps)
                }
            },
            Phase::Waiting(steps) => self.forward(steps.into_bound(py)),
            Phase::Done => Err(spent()),
        }
    }

    // The forwarded wait reaches back to the awaiting task through the
    // future's wakeup callback; without these the collector cannot see that
    // cycle and a task abandoned mid-wait would never be freed.
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Phase::Waiting(steps) = &self.phase {
            visit.call(steps)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        self.phase = Phase::Done;
    }
}

impl YieldFirstStep {
    fn forward(&mut self, steps: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let yielded = steps.call_method0(pyo3::intern!(steps.py(), "__next__"))?;
        self.phase = Phase::Waiting(steps.unbind());
        Ok(yielded.unbind())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    fn poll<S: Stream + Unpin>(stream: &mut S) -> Poll<Option<S::Item>> {
        poll_buffered(Pin::new(stream))
    }

    #[test]
    fn parked_item_comes_out_before_the_rest() {
        let mut parking = Parking::new(stream::iter([1, 2]));
        assert_eq!(poll(&mut parking), Poll::Ready(Some(1)));
        parking.park(Some(1));
        assert_eq!(poll(&mut parking), Poll::Ready(Some(1)));
        assert_eq!(poll(&mut parking), Poll::Ready(Some(2)));
        assert_eq!(poll(&mut parking), Poll::Ready(None));
    }

    #[test]
    fn parked_end_is_reported_once_more() {
        let mut parking = Parking::new(stream::iter([1]));
        parking.park(None);
        assert_eq!(poll(&mut parking), Poll::Ready(None));
        assert_eq!(poll(&mut parking), Poll::Ready(Some(1)));
    }

    #[test]
    fn streak_is_due_at_the_item_limit_however_items_arrive() {
        let mut streak = BufferedStreak::new();
        assert!(!streak.is_due());
        streak.extend_by(YIELD_INTERVAL - 1);
        // Under the limit and inside the time budget.
        assert!(!streak.is_due());
        streak.extend();
        assert!(streak.is_due());
        streak.reset();
        assert!(!streak.is_due());
    }

    #[test]
    fn streak_is_due_once_the_time_budget_is_spent() {
        let mut streak = BufferedStreak::new();
        streak.extend_by(BUDGET_CHECK_INTERVAL);
        std::thread::sleep(YIELD_BUDGET * 2);
        assert!(streak.is_due());
    }
}
