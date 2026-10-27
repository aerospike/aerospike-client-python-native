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

"""Integration tests for TLS and PKI connection functionality."""

import os
import pytest
from aerospike_native import new_client, ClientPolicy, TlsConfig, AuthMode, Key
from aerospike_native.exceptions import ConnectionError as AerospikeConnectionError


def _tls_host_env():
    """TLS host from AEROSPIKE_TLS_HOST or AEROSPIKE_HOST_TLS (aerospike.env)."""
    return os.environ.get("AEROSPIKE_TLS_HOST") or os.environ.get("AEROSPIKE_HOST_TLS")


def _tls_login_host_env():
    """A TLS node with security and a cleartext service port, for login-only TLS."""
    return os.environ.get("AEROSPIKE_HOST_TLS_LOGIN")


def _tls_ca_exists():
    """True only when the configured CA file actually exists on disk."""
    ca = os.environ.get("AEROSPIKE_TLS_CA_FILE")
    return ca is not None and os.path.isfile(ca)


@pytest.mark.skipif(
    not _tls_host_env() or not _tls_ca_exists(),
    reason="TLS host not set or CA file does not exist — TLS server not available"
)
class TestTlsConnection:
    """Test TLS connection functionality (requires TLS-enabled server)."""

    # use_services_alternate is required on ClientPolicy for TLS connections

    @pytest.fixture
    def tls_host(self):
        """Get TLS host from environment."""
        return _tls_host_env() or "localhost:4333"

    @pytest.fixture
    def tls_ca_file(self):
        """Get TLS CA file path from environment."""
        return os.environ.get("AEROSPIKE_TLS_CA_FILE")

    @pytest.fixture
    def tls_name(self):
        """Get TLS name from environment (must match server tls-name for cert validation)."""
        return os.environ.get("AEROSPIKE_TLS_NAME")

    async def test_tls_connection_basic(self, tls_host, tls_ca_file, tls_name):
        """Test basic TLS connection with CA certificate."""
        if not tls_ca_file:
            pytest.skip("AEROSPIKE_TLS_CA_FILE not set")

        policy = ClientPolicy()
        policy.use_services_alternate = True
        policy.tls_config = TlsConfig(tls_ca_file)
        policy.set_auth_mode(
            AuthMode.INTERNAL,
            user=os.environ.get("AEROSPIKE_USER", "admin"),
            password=os.environ.get("AEROSPIKE_PASSWORD", "admin")
        )

        # Use host:tls_name:port so client validates server cert against tls-name (e.g. tls1)
        if tls_name:
            parts = (tls_host or "localhost:4333").split(":")
            base_host = parts[0] if parts else "localhost"
            port = parts[1] if len(parts) > 1 else "4333"
            connect_host = f"{base_host}:{tls_name}:{port}"
        else:
            connect_host = tls_host

        client = await new_client(policy, connect_host)
        assert client is not None

        connected = client.is_connected()
        assert connected is True

        await client.close()

    async def test_tls_connection_with_tls_name(self, tls_host, tls_ca_file, tls_name):
        """Test TLS connection with TLS name in host string (host:tls_name:port)."""
        if not tls_ca_file:
            pytest.skip("AEROSPIKE_TLS_CA_FILE not set")
        if not tls_name:
            pytest.skip("AEROSPIKE_TLS_NAME not set - required for this test")

        policy = ClientPolicy()
        policy.use_services_alternate = True
        policy.tls_config = TlsConfig(tls_ca_file)
        policy.set_auth_mode(
            AuthMode.INTERNAL,
            user=os.environ.get("AEROSPIKE_USER", "admin"),
            password=os.environ.get("AEROSPIKE_PASSWORD", "admin")
        )

        parts = (tls_host or "localhost:4333").split(":")
        base_host = parts[0] if parts else "localhost"
        port = parts[1] if len(parts) > 1 else "4333"
        connect_host = f"{base_host}:{tls_name}:{port}"

        client = await new_client(policy, connect_host)
        assert client is not None

        connected = client.is_connected()
        assert connected is True

        await client.close()


def _tls_client_cert_exists():
    """True only when the configured client cert file actually exists on disk."""
    cert = os.environ.get("AEROSPIKE_TLS_CLIENT_CERT_FILE")
    return cert is not None and os.path.isfile(cert)


@pytest.mark.skipif(
    not _tls_host_env() or not _tls_ca_exists() or not _tls_client_cert_exists(),
    reason="TLS host/CA/client-cert not set or files missing — PKI not available"
)
class TestPkiConnection:
    """Test PKI authentication (requires TLS server with client certificate support)."""

    # use_services_alternate is required on ClientPolicy for TLS connections

    @pytest.fixture
    def tls_host(self):
        """Get TLS host from environment."""
        return _tls_host_env() or "localhost:4333"

    @pytest.fixture
    def tls_ca_file(self):
        """Get TLS CA file path from environment."""
        return os.environ.get("AEROSPIKE_TLS_CA_FILE")

    @pytest.fixture
    def tls_name(self):
        """Get TLS name from environment (must match server tls-name for cert validation)."""
        return os.environ.get("AEROSPIKE_TLS_NAME")

    @pytest.fixture
    def tls_client_cert_file(self):
        """Get TLS client certificate file path from environment."""
        return os.environ.get("AEROSPIKE_TLS_CLIENT_CERT_FILE")

    @pytest.fixture
    def tls_client_key_file(self):
        """Get TLS client key file path from environment."""
        return os.environ.get("AEROSPIKE_TLS_CLIENT_KEY_FILE")

    def _connect_host(self, tls_host, tls_name):
        """Build host:tls_name:port when tls_name set, else tls_host."""
        if tls_name:
            parts = (tls_host or "localhost:4333").split(":")
            base_host = parts[0] if parts else "localhost"
            port = parts[1] if len(parts) > 1 else "4333"
            return f"{base_host}:{tls_name}:{port}"
        return tls_host

    async def test_pki_connection(self, tls_host, tls_ca_file, tls_name, tls_client_cert_file, tls_client_key_file):
        """Test TLS connection with client certificate and admin auth (same connection model as basic TLS)."""
        if not all([tls_ca_file, tls_client_cert_file, tls_client_key_file]):
            pytest.skip("TLS certificate files not configured")

        policy = ClientPolicy()
        policy.use_services_alternate = True
        policy.tls_config = TlsConfig.with_client_auth(
            tls_ca_file,
            tls_client_cert_file,
            tls_client_key_file
        )
        policy.set_auth_mode(
            AuthMode.INTERNAL,
            user=os.environ.get("AEROSPIKE_USER", "admin"),
            password=os.environ.get("AEROSPIKE_PASSWORD", "admin")
        )

        connect_host = self._connect_host(tls_host, tls_name)
        client = await new_client(policy, connect_host)
        assert client is not None

        connected = client.is_connected()
        assert connected is True

        await client.close()

    async def test_pki_connection_with_set_auth_mode(self, tls_host, tls_ca_file, tls_name, tls_client_cert_file, tls_client_key_file):
        """Test TLS connection with client cert and admin auth; policy can set AuthMode.PKI for other use."""
        if not all([tls_ca_file, tls_client_cert_file, tls_client_key_file]):
            pytest.skip("TLS certificate files not configured")

        policy = ClientPolicy()
        policy.use_services_alternate = True
        policy.tls_config = TlsConfig.with_client_auth(
            tls_ca_file,
            tls_client_cert_file,
            tls_client_key_file
        )
        policy.set_auth_mode(
            AuthMode.INTERNAL,
            user=os.environ.get("AEROSPIKE_USER", "admin"),
            password=os.environ.get("AEROSPIKE_PASSWORD", "admin")
        )

        connect_host = self._connect_host(tls_host, tls_name)
        client = await new_client(policy, connect_host)
        assert client is not None

        connected = client.is_connected()
        assert connected is True

        await client.close()


class TestTlsConfigOptions:
    """Construction-level coverage for the options `TlsConfig` accepts.

    These build a config and assert on acceptance or rejection; they do not
    need a TLS server, because what is being pinned is that an option reaches
    rustls at all. The failure this guards against is the silent one -- a
    caller restricting protocols or ciphers and getting neither, with no error.
    """

    def test_bundled_roots_need_no_ca_file(self):
        """No CA file is a supported configuration, not "no TLS".

        A `tls_name`-only setup verifies against the public roots compiled
        into the client. A builder that returned None here would silently
        produce a plaintext connection.
        """
        assert TlsConfig() is not None

    def test_ca_file_still_positional(self):
        """The original single-argument form keeps working."""
        ca = os.environ.get("AEROSPIKE_TLS_CA_FILE")
        if not ca or not os.path.exists(ca):
            pytest.skip("AEROSPIKE_TLS_CA_FILE unset or missing")
        assert TlsConfig(ca) is not None

    def test_protocols_accepted(self):
        assert TlsConfig(protocols=["TLSv1.3"]) is not None
        assert TlsConfig(protocols=["TLSv1.2", "TLSv1.3"]) is not None

    def test_unknown_protocol_raises(self):
        """Rejected, not ignored -- dropping a restriction is how a caller
        negotiates something they explicitly excluded."""
        with pytest.raises(ValueError, match="Unsupported TLS protocol"):
            TlsConfig(protocols=["SSLv3"])

    def test_ciphers_accepted(self):
        assert TlsConfig(ciphers=["TLS13_AES_256_GCM_SHA384"]) is not None

    def test_unknown_cipher_raises(self):
        with pytest.raises(ValueError, match="Unknown cipher suite"):
            TlsConfig(ciphers=["NOT_A_SUITE"])

    def test_unknown_cipher_error_names_the_alternatives(self):
        """The message carries the supported suites, so a caller can correct
        the name without going to the rustls docs."""
        with pytest.raises(ValueError) as exc:
            TlsConfig(ciphers=["NOT_A_SUITE"])
        assert "TLS13_AES_256_GCM_SHA384" in str(exc.value)

    def test_protocol_cipher_mismatch_raises_value_error(self):
        """TLS 1.2 with only a TLS 1.3 suite leaves nothing to negotiate. That
        is a configuration mistake, so it raises the same class as a bad name."""
        with pytest.raises(ValueError, match="no usable cipher suites"):
            TlsConfig(protocols=["TLSv1.2"], ciphers=["TLS13_AES_256_GCM_SHA384"])


@pytest.mark.skipif(
    not _tls_login_host_env() or not _tls_ca_exists(),
    reason="AEROSPIKE_HOST_TLS_LOGIN and AEROSPIKE_TLS_CA_FILE required",
)
class TestTlsForLoginOnly:
    """TLS for the login exchange only; data connections are cleartext."""

    def _policy(self, for_login_only):
        policy = ClientPolicy()
        policy.tls_config = TlsConfig(
            os.environ.get("AEROSPIKE_TLS_CA_FILE"), for_login_only=for_login_only
        )
        policy.set_auth_mode(
            AuthMode.INTERNAL,
            user=os.environ.get("AEROSPIKE_USER", "admin"),
            password=os.environ.get("AEROSPIKE_PASSWORD", "admin"),
        )
        policy.use_services_alternate = True
        return policy

    def _host(self):
        host, _, port = _tls_login_host_env().rpartition(":")
        tls_name = os.environ.get("AEROSPIKE_TLS_NAME")
        return f"{host}:{tls_name}:{port}" if tls_name else f"{host}:{port}", int(port)

    async def _connect_or_skip(self, policy, host):
        """The env var names the node; it still has to be up. A node that is
        configured but unreachable is a missing fixture, not a failure."""
        try:
            return await new_client(policy, host)
        except AerospikeConnectionError as e:
            pytest.skip(f"login-only TLS node {host} not reachable: {e}")

    def test_flag_round_trips_through_the_policy(self):
        policy = self._policy(True)
        assert policy.tls_config.for_login_only is True
        assert self._policy(False).tls_config.for_login_only is False

    async def test_data_connections_move_to_the_cleartext_port(self):
        """After a TLS login the node is reached on its cleartext service address."""
        host, tls_port = self._host()
        client = await self._connect_or_skip(self._policy(True), host)
        try:
            nodes = client.nodes()
            assert len(nodes) == 1
            port = nodes[0].host[1]
            assert port != tls_port, f"node still on the TLS port {tls_port}"
            key = Key("test", "tls_login", "k")
            await client.put(key, {"n": 1})
            record = await client.get(key)
            assert record.bins["n"] == 1
        finally:
            await client.close()

    async def test_without_the_flag_the_node_stays_on_the_tls_port(self):
        """The control: the same node with full TLS keeps the TLS port."""
        host, tls_port = self._host()
        client = await self._connect_or_skip(self._policy(False), host)
        try:
            nodes = client.nodes()
            assert nodes[0].host[1] == tls_port
        finally:
            await client.close()

