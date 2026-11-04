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

"""Row N-17 against the ``cn-only`` profile, ``ee`` only. The topology is
N-6's, but the server certificate carries the tls-name only as its subject
CN, with no matching dNSName SAN. In its own profile so that a failing N-17
does not fail the other rows."""

import l2_capture as capture
import l2_rows as rows
import l2_topology as topo
from aerospike_native import Host, TlsConfig

PROFILE = " (DISCOVERY_SUBSET=ee, profile cn-only)"


async def test_n17_cn_only_certificate_is_rejected():
    """N-17: a certificate that carries the tls-name only as its subject CN
    is rejected. The TLS handshake with the seed fails on the tls-name and no
    node is created."""
    topology = topo.t()
    rows.skip_unless(
        topology.subset == "ee",
        f"N-17 is ee only: it needs server TLS and TLS listeners{PROFILE}; subset is {topology.subset}",
    )
    rows.skip_unless(
        topology.tls_cert_shape == "cn-only",
        f"N-17 needs DISCOVERY_TLS_CERT_SHAPE=cn-only, a server certificate with the tls-name only "
        f"as its CN{PROFILE}; shape is {topology.tls_cert_shape}",
    )
    assert topology.ca_cert, "AEROSPIKE_CA_CERT is not set; the ee topology mounts the server CA"
    node_list = await topo.nodes()
    first = node_list[0]
    assert first.tls_port > 0, f"No TLS listener for {first}"
    assert first.tls_name, f"No tls-name for {first}"

    policy = topo.policy()
    policy.tls_config = TlsConfig(topology.ca_cert)
    policy.address_translator = rows.port_map(topo.tls_port_map(node_list))
    capture.clear()
    error = await rows.connect_error(policy, [Host(topology.host, first.tls_port, first.tls_name)], "N-17")
    message = topo.messages(error)
    print(f"L2-N17-ERROR {message} cnFallbackWarnings={rows.cn_fallback_warnings()}")
    assert "NotValidForName" in message or "certificate" in message.lower(), message
