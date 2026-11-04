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

"""The L2 discovery rows: skipped without the topology, each row recording
the client's log and restoring the topology after it, pass or fail.

The rows share Toxiproxy and the cluster, so they must run one at a time
(no ``pytest-xdist``).
"""

import sys
from pathlib import Path

import pytest

_this_dir = str(Path(__file__).parent)
if _this_dir not in sys.path:
    sys.path.insert(0, _this_dir)

import l2_capture as capture  # noqa: E402
import l2_topology as topo  # noqa: E402


@pytest.fixture(autouse=True)
async def l2_row():
    if topo.topology() is None:
        pytest.skip(f"DISCOVERY_ENDPOINT_HOSTNAME is not set. {topo.RUN_HINT}")
    capture.install()
    try:
        yield
    finally:
        try:
            await topo.restore_all()
        finally:
            capture.stop()
