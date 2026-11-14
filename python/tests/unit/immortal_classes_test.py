# Copyright 2026 Aerospike, Inc.
#
# Portions may be licensed to Aerospike, Inc. under one or more contributor
# license agreements WHICH ARE COMPATIBLE WITH THE APACHE LICENSE, VERSION 2.0.
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not
# use this file except in compliance with the License. You may obtain a copy of
# the License at http:#www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under
# the License.

"""Classes are immortal on free-threaded builds whose object layout is known.

Freeing an instance releases a reference to its class, which on a
free-threaded build is a write every thread contends on. The extension makes
its classes immortal at import to avoid that, by writing the object header
the way the interpreter does. These tests keep that arrangement honest.
"""

import gc
import sys
import sysconfig
import threading

import pytest

import aerospike_native._native as native
from aerospike_native import Key

# Mirrors the interpreter versions the extension has been checked against.
CHECKED_VERSIONS = ((3, 14), (3, 15))
FREE_THREADED = bool(sysconfig.get_config_var("Py_GIL_DISABLED"))

needs_immortal_classes = pytest.mark.skipif(
    not FREE_THREADED or sys.version_info[:2] not in CHECKED_VERSIONS,
    reason="classes are made immortal only on free-threaded builds with a checked layout",
)


def _own_classes():
    return {
        name: value
        for name, value in vars(native).items()
        if isinstance(value, type) and value.__module__ in ("_native", "builtins")
    }


@pytest.mark.skipif(not FREE_THREADED, reason="only free-threaded builds are affected")
def test_new_interpreter_version_is_checked():
    assert sys.version_info[:2] <= CHECKED_VERSIONS[-1], (
        "This free-threaded Python is newer than the versions the extension makes its "
        "classes immortal on, so threads contend on every class again. Compare the "
        "interpreter's free-threaded object header and its routine for making an object "
        "immortal with src/immortal.rs, then add the version there and here."
    )


@needs_immortal_classes
def test_every_class_is_immortal():
    classes = _own_classes()
    assert len(classes) > 100
    mortal = sorted(name for name, cls in classes.items() if not sys._is_immortal(cls))
    assert not mortal, f"registered after the classes were made immortal: {mortal}"


@needs_immortal_classes
def test_stream_step_class_is_immortal():
    """One is created per row of an async stream, and it is in no namespace."""
    assert sys._is_immortal(type(native._ready_step(1)))


@needs_immortal_classes
def test_classes_outside_the_extension_are_left_alone():
    foreign = [
        value
        for value in vars(native.exceptions).values()
        if isinstance(value, type) and value.__module__ == "aerospike_native.exceptions"
    ]
    assert foreign
    assert not any(sys._is_immortal(cls) for cls in foreign)


@needs_immortal_classes
def test_instances_are_still_freed_across_threads_and_collections():
    def build_and_drop():
        for _ in range(200):
            keys = [Key("test", "users", i) for i in range(100)]
            del keys

    threads = [threading.Thread(target=build_and_drop) for _ in range(8)]
    for thread in threads:
        thread.start()
    gc.collect()
    for thread in threads:
        thread.join()
    gc.collect()

    assert Key("test", "users", 1).value == 1
    assert sys._is_immortal(Key)
