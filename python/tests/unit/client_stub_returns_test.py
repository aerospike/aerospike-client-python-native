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

"""An async ``Client`` method's stub awaits what its blocking twin returns.

An async method returns a Python awaitable built at runtime, so stub
generation cannot infer what it resolves to; each carries a hand-written
``#[gen_stub(override_return_type(...))]``. Its ``*_blocking`` twin returns the
same value directly, with a return type generated from the Rust signature, so
the twin is the reference the hand-written one is checked against.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

import aerospike_native

_STUB = Path(aerospike_native.__file__).with_name("_native.pyi").read_text()
_CLIENT = re.search(r"^class Client\b.*?(?=^class |\Z)", _STUB, re.S | re.M).group(0)
_RETURNS = dict(re.findall(r"^    def (\w+)\(self.*\) -> (.+?):(?: \.\.\.)?$", _CLIENT, re.M))
_PAIRS = sorted(name for name in _RETURNS if f"{name}_blocking" in _RETURNS)


def _normalized(annotation: str) -> str:
    """One spelling per type: the stub mixes ``typing.List`` and ``builtins.list``."""
    for old, new in (("builtins.", ""), ("typing.List", "list"), ("typing.Dict", "dict"), ("typing.", "")):
        annotation = annotation.replace(old, new)
    return re.sub(r"\s+", "", annotation)


@pytest.mark.parametrize("name", _PAIRS)
def test_async_method_awaits_what_its_blocking_twin_returns(name):
    expected = f"Awaitable[{_normalized(_RETURNS[f'{name}_blocking'])}]"
    assert _normalized(_RETURNS[name]) == expected


def test_client_has_async_methods_with_blocking_twins():
    """Guards the parametrized test above against collecting nothing."""
    assert len(_PAIRS) > 40
