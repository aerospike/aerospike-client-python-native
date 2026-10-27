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

"""Unit tests for Host and the seed argument of the client constructors."""

import pytest
from aerospike_native import ClientPolicy, Host, new_client_blocking


class TestHost:
    """Host construction, formatting, and value semantics."""

    def test_fields(self):
        host = Host("db1.example.com", 4333, tls_name="db1")
        assert (host.name, host.port, host.tls_name) == ("db1.example.com", 4333, "db1")

    def test_blank_tls_name_is_none(self):
        assert Host("db1", 3000, tls_name="  ").tls_name is None

    @pytest.mark.parametrize(
        ("host", "seed"),
        [
            (Host("10.0.0.1", 3000), "10.0.0.1:3000"),
            (Host("db1", 4333, tls_name="tls1"), "db1:tls1:4333"),
            (Host("fd00::2", 3000), "[fd00::2]:3000"),
            (Host("fd00::2", 4333, tls_name="tls1"), "[fd00::2]:tls1:4333"),
        ],
    )
    def test_str_is_seed_entry(self, host, seed):
        assert str(host) == seed

    def test_repr(self):
        assert repr(Host("fd00::2", 3000)) == "Host('fd00::2', 3000)"
        assert repr(Host("db1", 4333, tls_name="tls1")) == "Host('db1', 4333, tls_name='tls1')"

    def test_equality_and_hash(self):
        assert Host("db1", 3000) == Host("db1", 3000)
        assert Host("db1", 3000) != Host("db1", 3000, tls_name="tls1")
        assert len({Host("db1", 3000), Host("db1", 3000)}) == 1

    def test_immutable(self):
        with pytest.raises(AttributeError):
            Host("db1", 3000).port = 3001  # type: ignore[misc]

    @pytest.mark.parametrize("name", ["", "[fd00::2]"])
    def test_rejects_invalid_name(self, name):
        with pytest.raises(ValueError):
            Host(name, 3000)

    def test_rejects_out_of_range_port(self):
        with pytest.raises(OverflowError):
            Host("db1", 70000)


class TestSeedsArgument:
    """Seed values rejected before any connection attempt."""

    def test_empty_host_list(self):
        with pytest.raises(ValueError, match="at least one Host"):
            new_client_blocking(ClientPolicy(), [])

    @pytest.mark.parametrize("seeds", [["db1:3000"], 3000, None])
    def test_wrong_type(self, seeds):
        with pytest.raises(TypeError, match="str or a sequence of Host"):
            new_client_blocking(ClientPolicy(), seeds)
