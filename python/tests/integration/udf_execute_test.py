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

"""Tests for execute_udf functionality."""
import os
import pytest
from aerospike_native import WritePolicy, ReadPolicy, Key, ResultCode, UDFLang
from aerospike_native.exceptions import TimeoutError, UDFBadResponse
from fixtures import TestFixtureConnection


class TestExecuteUDF(TestFixtureConnection):
    """Test execute_udf functionality."""

    @pytest.fixture
    async def client_with_udf(self, client):
        """Register a test UDF for execute tests."""
        udf_path = os.path.join(os.path.dirname(__file__), "udf", "record_example.lua")
        server_path = "record_example.lua"

        # Clean up any existing UDF first
        try:
            remove_task = await client.remove_udf(server_path)
            await remove_task.wait_till_complete()
        except Exception:
            pass

        # Register the UDF
        task = await client.register_udf_from_file(udf_path, server_path, UDFLang.LUA)
        completed = await task.wait_till_complete()
        assert completed, f"UDF registration did not complete. Final status: {await task.query_status()}"

        yield client

        # Clean up
        try:
            remove_task = await client.remove_udf(server_path)
            await remove_task.wait_till_complete()
        except Exception:
            pass

    async def test_execute_udf_write_bin(self, client_with_udf):
        """Test executing UDF to write a bin."""
        key = Key("test", "test", "udfkey1")
        wp = WritePolicy()
        rp = ReadPolicy()

        await client_with_udf.delete(key, policy=wp)

        result = await client_with_udf.execute_udf(
            key,
            "record_example",
            "writeBin",
            ["udfbin1", "string value"],
            policy=wp,
        )
        assert result is None

        record = await client_with_udf.get(key, ["udfbin1"], policy=rp)
        assert record is not None
        assert record.bins["udfbin1"] == "string value"

    async def test_execute_udf_read_bin(self, client_with_udf):
        """Test executing UDF to read a bin."""
        key = Key("test", "test", "udfkey2")
        wp = WritePolicy()

        await client_with_udf.put(key, {"udfbin2": "test value"}, policy=wp)

        result = await client_with_udf.execute_udf(
            key,
            "record_example",
            "readBin",
            ["udfbin2"],
            policy=wp,
        )
        assert result == "test value"

    async def test_execute_udf_get_generation(self, client_with_udf):
        """Test executing UDF to get record generation."""
        key = Key("test", "test", "udfkey3")
        wp = WritePolicy()

        await client_with_udf.put(key, {"bin": "value"}, policy=wp)

        result = await client_with_udf.execute_udf(key, "record_example", "getGeneration", None, policy=wp)
        assert isinstance(result, int)
        assert result >= 0

    async def test_execute_udf_with_validation_success(self, client_with_udf):
        """Test executing UDF with validation that succeeds."""
        key = Key("test", "test", "udfkey4")
        wp = WritePolicy()

        await client_with_udf.delete(key, policy=wp)

        result = await client_with_udf.execute_udf(
            key,
            "record_example",
            "writeWithValidation",
            ["udfbin4", 5],
            policy=wp,
        )
        assert result is None

    async def test_execute_udf_with_validation_failure(self, client_with_udf):
        """Test executing UDF with validation that fails."""
        key = Key("test", "test", "udfkey5")
        wp = WritePolicy()

        await client_with_udf.delete(key, policy=wp)

        with pytest.raises(UDFBadResponse):
            await client_with_udf.execute_udf(
                key,
                "record_example",
                "writeWithValidation",
                ["udfbin5", 11],
                policy=wp,
            )

    async def test_execute_udf_write_unique_success(self, client_with_udf):
        """Test executing UDF writeUnique when record doesn't exist."""
        key = Key("test", "test", "udfkey6")
        wp = WritePolicy()
        rp = ReadPolicy()

        await client_with_udf.delete(key, policy=wp)

        result = await client_with_udf.execute_udf(
            key,
            "record_example",
            "writeUnique",
            ["udfbin6", "first"],
            policy=wp,
        )
        assert result is None

        record = await client_with_udf.get(key, ["udfbin6"], policy=rp)
        assert record is not None
        assert record.bins["udfbin6"] == "first"

    async def test_execute_udf_write_unique_failure(self, client_with_udf):
        """Test executing UDF writeUnique when record already exists."""
        key = Key("test", "test", "udfkey7")
        wp = WritePolicy()
        rp = ReadPolicy()

        await client_with_udf.delete(key, policy=wp)
        await client_with_udf.put(key, {"udfbin7": "first"}, policy=wp)

        result = await client_with_udf.execute_udf(
            key,
            "record_example",
            "writeUnique",
            ["udfbin7", "second"],
            policy=wp,
        )
        assert result is None

        record = await client_with_udf.get(key, ["udfbin7"], policy=rp)
        assert record.bins["udfbin7"] == "first"

    async def test_execute_udf_no_args(self, client_with_udf):
        """Test executing UDF with no arguments."""
        key = Key("test", "test", "udfkey8")
        wp = WritePolicy()

        await client_with_udf.delete(key, policy=wp)

        result = await client_with_udf.execute_udf(
            key,
            "record_example",
            "getGeneration",
            None,
            policy=wp,
        )
        assert isinstance(result, int)

    async def test_execute_udf_nonexistent_function(self, client_with_udf):
        """Test executing UDF with non-existent function name."""
        key = Key("test", "test", "udfkey9")
        wp = WritePolicy()

        with pytest.raises(UDFBadResponse):
            await client_with_udf.execute_udf(
                key,
                "record_example",
                "nonexistentFunction",
                None,
                policy=wp,
            )

    async def test_execute_udf_nonexistent_module(self, client_with_udf):
        """Test executing UDF with non-existent module."""
        key = Key("test", "test", "udfkey10")
        wp = WritePolicy()

        with pytest.raises(UDFBadResponse):
            await client_with_udf.execute_udf(
                key,
                "nonexistent",
                "writeBin",
                ["bin", "value"],
                policy=wp,
            )

    @pytest.fixture
    async def client_with_sleep_udf(self, client):
        """Register sleep UDF for timeout tests."""
        udf_path = os.path.join(os.path.dirname(__file__), "udf", "sleep_example.lua")
        server_path = "sleep_example.lua"

        # Clean up any existing UDF first
        try:
            remove_task = await client.remove_udf(server_path)
            await remove_task.wait_till_complete()
        except Exception:
            pass

        # Register the UDF
        task = await client.register_udf_from_file(udf_path, server_path, UDFLang.LUA)
        completed = await task.wait_till_complete()
        assert completed, f"UDF registration did not complete. Final status: {await task.query_status()}"

        yield client

        # Clean up
        try:
            remove_task = await client.remove_udf(server_path)
            await remove_task.wait_till_complete()
        except Exception:
            pass

    async def test_udf_timeout_handling(self, client_with_sleep_udf):
        """Test that total_timeout handles UDF timeouts (may be server-side UDFBadResponse or client-side TimeoutError)."""
        from aerospike_native.exceptions import TimeoutError, UDFBadResponse

        key = Key("test", "test", "timeout_test_key")
        wp = WritePolicy()

        # Set a short total_timeout (500ms)
        wp.total_timeout = 500

        # Execute UDF that sleeps for longer than the timeout (2000ms = 2 seconds)
        # This should trigger a timeout error
        # Note: Server may return UDFBadResponse with "UDF Execution Timeout" message
        # or client may raise TimeoutError - both indicate timeout enforcement
        with pytest.raises((TimeoutError, UDFBadResponse)):
            await client_with_sleep_udf.execute_udf(
                key,
                "sleep_example",
                "sleep",
                [2000],
                policy=wp,
            )

    async def test_udf_client_timeout_marks_in_doubt(self, client_with_sleep_udf):
        """A client socket timeout on a write that reached the server marks the error in-doubt."""
        from aerospike_native.exceptions import TimeoutError

        key = Key("test", "test", "in_doubt_test_key")
        wp = WritePolicy()

        # The socket timeout must fire while the server is still executing the
        # UDF: the write reached the wire but its outcome is unknown, which is
        # exactly the in-doubt condition. total_timeout must stay 0: a nonzero
        # total makes the client send min(socket, total) as the server-side
        # deadline, and the server's own "UDF Execution Timeout" abort then
        # beats the client's socket timer. With no server deadline the client's
        # 250ms timer is the only one racing the 1000ms UDF sleep, so the
        # client-side timeout is deterministic even under server load.
        wp.socket_timeout = 250
        wp.total_timeout = 0
        wp.max_retries = 0

        with pytest.raises(TimeoutError) as exc_info:
            await client_with_sleep_udf.execute_udf(
                key,
                "sleep_example",
                "sleep",
                [1000],
                policy=wp,
            )
        assert exc_info.value.in_doubt is True

    async def test_udf_timeout_not_triggered_on_fast_operation(self, client_with_sleep_udf):
        """Test that total_timeout doesn't trigger on fast UDF operations."""
        key = Key("test", "test", "timeout_test_key2")
        wp = WritePolicy()

        # Set a total_timeout (1000ms)
        wp.total_timeout = 1000

        # Execute UDF that sleeps for shorter than the timeout (100ms)
        # This should complete successfully
        result = await client_with_sleep_udf.execute_udf(
            key,
            "sleep_example",
            "sleep",
            [100],
            policy=wp,
        )
        assert result == "slept"

    async def test_udf_client_timeout_carries_retry_context(self, client_with_sleep_udf):
        """A client timeout surfaces the retry context: node, iteration,
        base_message, and — when the retry loop re-attempted — the prior
        attempts as sub_exceptions.

        Same deterministic shape as the in-doubt test above (socket timeout
        racing a longer server-side UDF sleep, no server deadline), with
        retries enabled so the context has more than one attempt to record.
        """
        from aerospike_native.exceptions import TimeoutError

        key = Key("test", "test", "retry_ctx_test_key")
        wp = WritePolicy()
        wp.socket_timeout = 250
        wp.total_timeout = 0
        wp.max_retries = 2

        with pytest.raises(TimeoutError) as exc_info:
            await client_with_sleep_udf.execute_udf(
                key,
                "sleep_example",
                "sleep",
                [1000],
                policy=wp,
            )
        err = exc_info.value
        assert err.in_doubt is True
        assert err.node, "expected the attempted node to be recorded"
        assert err.iteration is not None and err.iteration >= 1
        assert err.base_message
        # Timeouts are retriable, so the prior attempts must be recorded as
        # typed exception instances. In-doubt is a command-level verdict
        # (finalized on the top error only), so it is not asserted per attempt.
        assert err.sub_exceptions, "expected prior attempts in sub_exceptions"
        assert all(isinstance(s, TimeoutError) for s in err.sub_exceptions)

    async def test_udf_retried_timeout_chains_the_last_attempt_on_cause(self, client_with_sleep_udf):
        """A retried timeout reports MAX_RETRIES_EXCEEDED over the last
        attempt's own timeout, which ``__cause__`` carries as an exception
        with its own one-line message."""
        key = Key("test", "test", "retry_cause_test_key")
        wp = WritePolicy()
        wp.socket_timeout = 250
        wp.total_timeout = 0
        wp.max_retries = 1

        with pytest.raises(TimeoutError) as exc_info:
            await client_with_sleep_udf.execute_udf(
                key,
                "sleep_example",
                "sleep",
                [1000],
                policy=wp,
            )
        err = exc_info.value
        cause = err.__cause__
        assert err.result_code == ResultCode.MAX_RETRIES_EXCEEDED
        assert isinstance(cause, TimeoutError)
        assert cause.result_code == ResultCode.TIMEOUT
        assert "\n" not in str(cause)
        assert cause.__cause__ is None
