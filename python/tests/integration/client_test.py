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

import os

import pytest

from aerospike_native import new_client, ClientPolicy, Host
from aerospike_native.exceptions import ConnectionError, ResultCode

async def test_connect():
    """Test basic client connection."""

    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, os.environ.get("AEROSPIKE_HOST", "localhost:3000"))
    assert client is not None

async def test_connect_with_hosts():
    """A list of Host connects, and seeds() reports it in seed-string form."""
    name, port = os.environ.get("AEROSPIKE_HOST", "localhost:3000").rsplit(":", 1)
    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, [Host(name, int(port))])
    try:
        assert client.is_connected() is True
        assert client.seeds() == f"{name}:{port}"
    finally:
        await client.close()

async def test_failed_connect():
    """Test basic client connection failure."""
    client = None
    cp = ClientPolicy()
    cp.use_services_alternate = True
    # Set a shorter timeout for faster test execution
    cp.timeout = 2000  # 2 seconds instead of default 30 seconds
    # Use a non-existent host and port to ensure connection failure
    with pytest.raises(ConnectionError) as exc_info:
        client = await new_client(cp, "nonexistent-host:9999")
    assert client is None
    # The core error Display is now uniform and Java-style:
    # "Error <code>: Unable to communicate with server cluster: Failed to
    # connect to [N] host(s):\n  <host> <err>\n…", followed by the cause
    # chain (e.g. the DNS lookup failure). The stable, meaningful part is
    # the seed-failure breakdown, which is present regardless of how many
    # seeds failed — assert on that substring rather than a fixed prefix.
    assert "Failed to connect to" in exc_info.value.args[0]
    # A client-side failure carries its code, the same negative number
    # every Aerospike client reports for an unreachable cluster.
    assert exc_info.value.result_code == ResultCode.SERVER_NOT_AVAILABLE

async def test_close():
    """Test client connection and proper closing."""

    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, os.environ.get("AEROSPIKE_HOST", "localhost:3000"))
    assert client is not None
    await client.close()


async def test_is_connected():
    """Test is_connected() method returns True when connected and False after closing."""
    cp = ClientPolicy()
    cp.use_services_alternate = True
    client = await new_client(cp, os.environ.get("AEROSPIKE_HOST", "localhost:3000"))
    assert client is not None

    # After successful connection, should be connected
    connected = client.is_connected()
    assert connected is True, "Client should be connected after successful new_client()"

    # Close the client (now async)
    await client.close()

    # After closing, should not be connected
    connected = client.is_connected()
    assert connected is False, "Client should not be connected after close()"
