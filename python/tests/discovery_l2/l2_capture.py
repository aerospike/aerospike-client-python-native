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

"""The client's warnings and errors, recorded for the row that runs."""

import logging
import threading

# core's log records reach Python logging under its crate name.
LOGGERS = ("aerospike_core", "aerospike_native")


class _Capture(logging.Handler):
    def __init__(self) -> None:
        super().__init__(logging.WARNING)
        self.lines: list[tuple[int, str]] = []
        self.lines_lock = threading.Lock()

    def emit(self, record: logging.LogRecord) -> None:
        message = record.getMessage()
        print(f"CLIENT-LOG {record.levelname} {message}")
        with self.lines_lock:
            self.lines.append((record.levelno, message))


_CAPTURE = _Capture()


def install() -> None:
    """Starts recording, with nothing recorded."""
    clear()
    for name in LOGGERS:
        logging.getLogger(name).addHandler(_CAPTURE)


def stop() -> None:
    for name in LOGGERS:
        logging.getLogger(name).removeHandler(_CAPTURE)


def clear() -> None:
    with _CAPTURE.lines_lock:
        _CAPTURE.lines.clear()


def _lines(level: int) -> list[str]:
    with _CAPTURE.lines_lock:
        return [m for lv, m in _CAPTURE.lines if lv == level]


def warnings() -> list[str]:
    return _lines(logging.WARNING)


def errors() -> list[str]:
    return _lines(logging.ERROR)


def messages_containing(*parts: str) -> list[str]:
    """The warnings, then the errors, that contain every one of ``parts``."""
    return [m for m in warnings() + errors() if all(p in m for p in parts)]
