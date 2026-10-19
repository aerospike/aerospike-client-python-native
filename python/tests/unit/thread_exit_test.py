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

"""Using the module after the thread that first imported it has exited.

mimalloc 3.3.x treats the first thread that initializes it as the main
thread, and the next allocation on a new thread segfaults once that thread
has exited. The test suite imports on the main thread, which never exits,
so only a fresh interpreter can exercise this path.
"""

import subprocess
import sys
import textwrap


_REPRO = textwrap.dedent("""\
    import threading

    modules = {}

    def first_import():
        import aerospike_native
        modules["native"] = aerospike_native

    loader = threading.Thread(target=first_import)
    loader.start()
    loader.join()
    native = modules["native"]

    def build_keys():
        for _ in range(300):
            [native.Key("test", "users", f"user-{i}") for i in range(50)]
            native.ClientPolicy()

    workers = [threading.Thread(target=build_keys) for _ in range(8)]
    for worker in workers:
        worker.start()
    for worker in workers:
        worker.join()
    print("OK")
""")


def test_use_after_importing_thread_exits():
    result = subprocess.run(
        [sys.executable, "-c", _REPRO],
        capture_output=True,
        text=True,
        timeout=60,
    )
    # A crash shows up only as a negative returncode (killed by signal).
    assert result.returncode == 0, (
        f"returncode={result.returncode} (negative => killed by signal)\n"
        f"--- stderr ---\n{result.stderr}"
    )
    assert result.stdout.strip() == "OK"
