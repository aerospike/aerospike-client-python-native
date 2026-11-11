# Copyright 2025-2026 Aerospike, Inc.
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

"""Unit tests for the internal completed-awaitable helper."""

import pytest

from aerospike_native import _ready_step


async def test_resolves_to_the_value_without_suspending():
    value = object()
    steps = _ready_step(value).__await__()
    with pytest.raises(StopIteration) as stop:
        next(steps)
    assert stop.value.value is value


async def test_tuple_value_is_not_unpacked():
    assert await _ready_step((1, 2)) == (1, 2)


async def test_none_is_a_value():
    assert await _ready_step(None) is None


async def test_cannot_be_awaited_twice():
    step = _ready_step(1)
    assert await step == 1
    with pytest.raises(RuntimeError):
        await step
