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

"""Tests for get_node, nodes(), and Node.info functionality."""

import pytest
from aerospike_native import Node
from aerospike_native.exceptions import InvalidNodeError
from fixtures import TestFixtureConnection


class TestGetNode(TestFixtureConnection):
    """Test get_node and Node.info functionality."""

    async def test_get_node_by_name(self, client):
        """Test getting a node by its name."""
        # First get the list of node names
        node_names = client.node_names()
        assert len(node_names) > 0, "Should have at least one node"

        # Get the first node by name
        node_name = node_names[0]
        node = client.get_node(node_name)
        assert node is not None

    async def test_node_info_build(self, client):
        """Test Node.info command for build information."""
        node_names = client.node_names()
        node : Node = client.get_node(node_names[0])

        response = await node.info("build")

        assert isinstance(response, dict)
        assert len(response) > 0, "Build info should contain data"

    async def test_node_info_namespaces(self, client):
        """Test Node.info command for namespaces."""
        node_names = client.node_names()
        node = client.get_node(node_names[0])

        response = await node.info("namespaces")

        assert isinstance(response, dict)
        assert len(response) > 0, "Namespaces info should contain data"

    async def test_node_info_statistics(self, client):
        """Test Node.info command for statistics."""
        node_names = client.node_names()
        node = client.get_node(node_names[0])

        response = await node.info("statistics")

        assert isinstance(response, dict)
        assert len(response) > 0, "Statistics should contain data"

    async def test_get_node_invalid_name(self, client):
        """Test getting a node with invalid name raises error."""
        with pytest.raises(InvalidNodeError):
            client.get_node("nonexistent_node_name_12345")

    async def test_multiple_nodes_info(self, client):
        """Test getting info from multiple nodes."""
        node_names = client.node_names()

        for name in node_names:
            node = client.get_node(name)
            response = await node.info("build")
            assert isinstance(response, dict)
            assert len(response) > 0


class TestNodeProperties(TestFixtureConnection):
    """Test Node properties: name, address, is_active, version, host."""

    async def test_node_name_property(self, client):
        """Test Node.name property returns a string."""
        nodes = client.nodes()
        node = nodes[0]

        assert isinstance(node.name, str)
        assert len(node.name) > 0

    async def test_node_address_property(self, client):
        """Test Node.address property returns address string."""
        nodes = client.nodes()
        node = nodes[0]

        assert isinstance(node.address, str)
        assert ":" in node.address  # Should be host:port format

    async def test_node_is_active_property(self, client):
        """Test Node.is_active property returns boolean."""
        nodes = client.nodes()
        node = nodes[0]

        assert isinstance(node.is_active, bool)
        assert node.is_active is True  # Node from active client should be active

    async def test_node_host_property(self, client):
        """Test Node.host property returns (hostname, port) tuple."""
        nodes = client.nodes()
        node = nodes[0]

        host = node.host
        assert isinstance(host, tuple)
        assert len(host) == 2
        assert isinstance(host[0], str)  # hostname
        assert isinstance(host[1], int)  # port

    async def test_node_version_property(self, client):
        """Test Node.version property returns Version object."""

        nodes = client.nodes()
        node = nodes[0]

        version = node.version
        assert version is not None
        assert isinstance(version.major, int)
        assert isinstance(version.minor, int)
        assert isinstance(version.patch, int)
        assert isinstance(version.build, int)

    async def test_node_str_repr(self, client):
        """Test Node __str__ and __repr__ methods."""
        nodes = client.nodes()
        node = nodes[0]

        node_str = str(node)
        node_repr = repr(node)

        assert "Node" in node_str
        assert node.name in node_str
        assert "Node" in node_repr
        assert node.name in node_repr


class TestVersion(TestFixtureConnection):
    """Test Version class properties and feature detection methods."""

    async def test_version_properties(self, client):
        """Test Version major, minor, patch, build properties."""
        nodes = client.nodes()
        version = nodes[0].version

        assert version.major >= 0
        assert version.minor >= 0
        assert version.patch >= 0
        assert version.build >= 0

    async def test_version_str_repr(self, client):
        """Test Version __str__ and __repr__ methods."""
        nodes = client.nodes()
        version = nodes[0].version

        version_str = str(version)
        version_repr = repr(version)

        # Should be in format "major.minor.patch.build"
        parts = version_str.split(".")
        assert len(parts) == 4
        assert "Version" in version_repr

    async def test_version_supports_query_selection(self, client):
        """Test Version.supports_query_selection() method."""
        nodes = client.nodes()
        version = nodes[0].version

        result = version.supports_query_selection()
        assert isinstance(result, bool)
        if (version.major, version.minor, version.patch) >= (8, 2, 0):
            assert result is True

    async def test_version_supports_vector_and_top_k(self, client):
        """Test Version.supports_vector() and supports_query_top_k() (server 8.2.1+)."""
        version = client.nodes()[0].version
        expected = (version.major, version.minor, version.patch) >= (8, 2, 1)
        assert version.supports_vector() is expected
        assert version.supports_query_top_k() is expected

    async def test_version_supports_server_compiled_ael(self, client):
        """Test Version.supports_server_compiled_ael() method."""
        nodes = client.nodes()
        version = nodes[0].version

        result = version.supports_server_compiled_ael()
        assert isinstance(result, bool)
        if (version.major, version.minor, version.patch) >= (8, 2, 0):
            assert result is True


class TestNodeMonitoring(TestFixtureConnection):
    """Test Node monitoring properties: failures, partition_generation, rebalance_generation, aliases."""

    async def test_node_failures_property(self, client):
        """Test Node.failures property returns connection failure count."""
        nodes = client.nodes()
        node = nodes[0]

        assert isinstance(node.failures, int)
        assert node.failures >= 0

    async def test_node_partition_generation_property(self, client):
        """Test Node.partition_generation property."""
        nodes = client.nodes()
        node = nodes[0]

        assert isinstance(node.partition_generation, int)

    async def test_node_rebalance_generation_property(self, client):
        """Test Node.rebalance_generation property."""
        nodes = client.nodes()
        node = nodes[0]

        assert isinstance(node.rebalance_generation, int)

    async def test_node_aliases_method(self, client):
        """Test Node.aliases() returns list of host tuples."""
        nodes = client.nodes()
        node = nodes[0]

        aliases = node.aliases()

        assert isinstance(aliases, list)
        assert len(aliases) >= 1  # At least the primary address
        for alias in aliases:
            assert isinstance(alias, tuple)
            assert len(alias) == 2
            assert isinstance(alias[0], str)  # hostname
            assert isinstance(alias[1], int)  # port


class TestNodes(TestFixtureConnection):
    """Test nodes() functionality.

    """

    async def test_nodes_returns_all_nodes(self, client):
        """Test nodes() returns all Node objects."""
        nodes = client.nodes()

        assert isinstance(nodes, list)
        assert len(nodes) > 0, "Should have at least one node"

        # Each item should be a Node object that can execute info commands
        for node in nodes:
            assert node is not None
            response = await node.info("build")
            assert isinstance(response, dict)

    async def test_nodes_first_element_access(self, client):
        """Test accessing first node by index."""
        nodes = client.nodes()
        assert len(nodes) > 0

        first_node = nodes[0]
        response = await first_node.info("namespaces")
        assert isinstance(response, dict)

    async def test_nodes_matches_node_names_count(self, client):
        """Test nodes() returns same count as node_names()."""
        nodes = client.nodes()
        node_names = client.node_names()

        assert len(nodes) == len(node_names)

    async def test_nodes_iterate_for_cluster_verification(self, client):
        """Test iterating over nodes for cluster-wide verification."""
        nodes = client.nodes()

        # Verify all nodes respond to info command
        for node in nodes:
            response = await node.info("statistics")
            assert isinstance(response, dict)
            assert len(response) > 0, "Each node should return statistics"

    async def test_nodes_for_cluster_aware_calculations(self, client):
        """Test using nodes() length for cluster-aware calculations."""
        nodes = client.nodes()
        records_per_node = 100

        # Calculate total records based on cluster size
        total_records = records_per_node * len(nodes)
        assert total_records >= records_per_node, "Should have at least records_per_node records"


class TestNodeErrorCases(TestFixtureConnection):
    """Test error handling for Node and related methods."""

    async def test_node_info_invalid_command_returns_error_response(self, client):
        """Test that Node.info() with invalid command returns error in response."""
        nodes = client.nodes()
        node = nodes[0]

        response = await node.info("invalid_command_xyz123")

        # Server returns error in response dict rather than raising exception
        assert isinstance(response, dict)
        assert "invalid_command_xyz123" in response
        assert "ERROR" in response["invalid_command_xyz123"]
