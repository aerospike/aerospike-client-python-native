# Copyright 2023-2026 Aerospike, Inc.
#
# Portions may be licensed to Aerospike, Inc. under one or more contributor
# license agreements WHICH ARE COMPATIBLE WITH THE APACHE LICENSE, VERSION 2.0.
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not
# use this file except in compliance with the License. You may obtain a copy of
# the License at http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under
# the License.

"""Tasks that run a command until stopped and record every outcome, for the
fault-injection rows."""

import asyncio
import time
from dataclasses import dataclass
from typing import Awaitable, Callable, Optional

from aerospike_native import ResultCode, exceptions

# Outcomes kept at all. Past this the counts go on and the list stops: a
# command that fails at once, with no node to send it to, completes at the
# rate the CPU allows.
KEPT_OUTCOMES = 200_000


def causes(error: Optional[BaseException]) -> list[BaseException]:
    """The error and every error beneath it on ``__cause__``."""
    chain = []
    while error is not None:
        chain.append(error)
        error = error.__cause__
    return chain


def attempts(error: BaseException) -> list[BaseException]:
    """The error, the errors of its earlier attempts, and their causes. A
    refusal or timeout that a retry wrapper reported over sits on the
    ``__cause__`` chain."""
    return [
        link
        for attempt in [error, *(getattr(error, "sub_exceptions", None) or [])]
        for link in causes(attempt)
    ]


def refused_by_breaker(error: BaseException) -> bool:
    """Whether an attempt was refused by the node's error-rate circuit breaker."""
    return any(
        isinstance(e, exceptions.MaxErrorRate) or getattr(e, "result_code", None) == ResultCode.MAX_ERROR_RATE
        for e in attempts(error)
    )


@dataclass(frozen=True)
class Outcome:
    """One command run by a :class:`Load`: when it started (monotonic
    seconds), how long it took, its error kind (``ok`` on success) and
    whether the circuit breaker refused it."""

    start: float
    elapsed_ms: int
    kind: str
    max_error_rate: bool

    @classmethod
    def of(cls, start: float, elapsed_ms: int, error: Optional[BaseException]) -> "Outcome":
        if error is None:
            return cls(start, elapsed_ms, "ok", False)
        return cls(
            start,
            elapsed_ms,
            f"{type(error).__name__}({getattr(error, 'result_code', None)})",
            refused_by_breaker(error),
        )

    @property
    def is_error(self) -> bool:
        return self.kind != "ok"


class Load:
    """Tasks that run a command until stopped."""

    def __init__(self, name: str, tasks: int, command: Callable[[int, int], Awaitable[object]]) -> None:
        """Starts ``tasks`` tasks, each awaiting ``command(task, iteration)``
        in a loop until :meth:`stop`."""
        self.name = name
        self.outcomes: list[Outcome] = []
        self.total = 0
        self.error_count = 0
        self._stopped = False
        self._tasks = [asyncio.create_task(self._run(task, command)) for task in range(tasks)]

    async def _run(self, task: int, command: Callable[[int, int], Awaitable[object]]) -> None:
        iteration = 0
        while not self._stopped:
            start = time.monotonic()
            error = None
            try:
                await command(task, iteration)
            except Exception as e:
                error = e
            elapsed_ms = int((time.monotonic() - start) * 1000)
            if error is not None:
                self.error_count += 1
            if self.total < KEPT_OUTCOMES:
                self.outcomes.append(Outcome.of(start, elapsed_ms, error))
            self.total += 1
            iteration += 1
            # A command refused without ever pending would otherwise keep this
            # task on the loop for good and starve the row's own timers.
            await asyncio.sleep(0)

    async def stop(self, bound_ms: int) -> bool:
        """Stops the tasks and returns whether every one of them returned
        within ``bound_ms``. A task still running past the bound is cancelled
        and named. Stopping a stopped load returns ``True``."""
        self._stopped = True
        if not self._tasks:
            return True
        _, pending = await asyncio.wait(self._tasks, timeout=bound_ms / 1000)
        for i, task in enumerate(self._tasks):
            if task in pending:
                print(f"L2-BLOCKED {self.name}-{i} still running {bound_ms}ms after stop")
                task.cancel()
        self._tasks = []
        return not pending

    def errors(self) -> list[Outcome]:
        return [o for o in self.outcomes if o.is_error]

    @property
    def successes(self) -> int:
        return self.total - self.error_count


def kinds(outcomes: list[Outcome]) -> dict[str, int]:
    """The number of outcomes of each error kind."""
    counts: dict[str, int] = {}
    for outcome in outcomes:
        counts[outcome.kind] = counts.get(outcome.kind, 0) + 1
    return dict(sorted(counts.items()))
