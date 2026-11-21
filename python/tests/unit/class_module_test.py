# Copyright 2025-2026 Aerospike, Inc.
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

"""Every class reports the module it really lives in.

A class's ``#[pyclass(module = ...)]`` sets its runtime ``__module__`` and is
also where stub generation takes the class's module from. A class declared
without one still gets a correct stub, from the package default, while at
runtime it reports ``builtins``, so nothing fails and nothing is noticed.
"""

from __future__ import annotations

import inspect
import re
from pathlib import Path

import pytest

import aerospike_native
from aerospike_native import _native

EXTENSION_MODULE = "aerospike_native._native"
EXCEPTIONS_MODULE = "aerospike_native.exceptions"

_CLASSES = sorted(
    (name, cls) for name, cls in vars(_native).items() if inspect.isclass(cls)
)


def _is_exception(cls: type) -> bool:
    return issubclass(cls, BaseException)


@pytest.mark.parametrize(
    "cls", [cls for _, cls in _CLASSES if not _is_exception(cls)], ids=lambda cls: cls.__name__
)
def test_class_reports_the_extension_module(cls):
    assert cls.__module__ == EXTENSION_MODULE, (
        f"{cls.__name__} reports module {cls.__module__!r}; "
        f'declare it with #[pyclass(module = "{EXTENSION_MODULE}")]'
    )


@pytest.mark.parametrize(
    "cls", [cls for _, cls in _CLASSES if _is_exception(cls)], ids=lambda cls: cls.__name__
)
def test_exception_reports_the_exceptions_module(cls):
    assert cls.__module__ == EXCEPTIONS_MODULE


def test_the_extension_module_defines_classes():
    """Guards the parametrized tests above against collecting nothing."""
    assert len(_CLASSES) > 100


def test_stub_names_no_type_through_a_bare_native_module():
    """``_native.X`` in the stub resolves to nothing: no such top-level module."""
    stub = Path(aerospike_native.__file__).with_name("_native.pyi").read_text()
    assert not re.findall(r"(?<![\w.])_native\.\w+", stub)
