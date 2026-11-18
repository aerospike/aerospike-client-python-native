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

import pytest
from aerospike_native import Key


def test_key_eq():
    """Test key equality comparisons."""

    key1 = Key("test", "test", 1)
    key2 = Key("test", "test", 1)
    key3 = Key("test", "test", 2)
    key4 = Key("test1", "test", 1)
    key5 = Key("test", "test1", 1)

    # because key1 and key2 have matching set ("test") and user_key (1)
    assert key1 == key2

    # because key1's user_key (1) does not match key3's user_key (2)
    assert key1 != key3

    # because key1's and key4's set ("test") and user_key (1) match:
    assert key1 == key4

    # because key1's set ("test") does not match key5's set ("test1"):
    assert key1 != key5

def test_key_hash_follows_equality():
    """Equal keys hash equal, so keys work as set members and dict keys."""
    key = Key("test", "test", 1)
    same = Key("test", "test", 1)
    from_digest = Key.key_with_digest("test", "test", key.digest)
    other = Key("test", "test", 2)

    assert hash(key) == hash(same) == hash(from_digest)
    assert len({key, same, from_digest, other}) == 2
    assert {key: "first"}[same] == "first"

def test_key_eq_with_non_key():
    """Comparing with a non-Key is unequal rather than an error."""
    key = Key("test", "test", 1)
    assert key != "test"
    assert not (key == 1)

def test_key_with_digest():
    """Test creating a key from a digest."""
    # Create a regular key first to get its digest
    original_key = Key("test", "test", 1)
    digest_hex = original_key.digest
    assert digest_hex is not None

    # Create a key from the digest (hex string)
    key_from_digest_hex = Key.key_with_digest("test", "test", digest_hex)
    assert key_from_digest_hex == original_key
    assert key_from_digest_hex.namespace == "test"
    assert key_from_digest_hex.set_name == "test"
    assert key_from_digest_hex.value is None  # No user key when created from digest

    # Create a key from the digest (bytes)
    digest_bytes = bytes.fromhex(digest_hex)
    key_from_digest_bytes = Key.key_with_digest("test", "test", digest_bytes)
    assert key_from_digest_bytes == original_key
    assert key_from_digest_bytes == key_from_digest_hex

def test_key_types():
    """Test creating keys with different value types."""
    # Integer key (preserved as integer - core client supports integer keys)
    key_int = Key("ns", "set", 12345)
    assert key_int.value == 12345
    assert isinstance(key_int.value, int)

    # String key
    key_str = Key("ns", "set", "string_key")
    assert key_str.value == "string_key"

    # Bytes key
    key_bytes = Key("ns", "set", b"bytes_key")
    assert key_bytes.value == b"bytes_key"

    # Empty string key
    key_empty = Key("ns", "set", "")
    assert key_empty.value == ""

    # Zero key (preserved as integer - core client supports integer keys)
    key_zero = Key("ns", "set", 0)
    assert key_zero.value == 0
    assert isinstance(key_zero.value, int)

def test_key_properties():
    """Test key properties are correctly set and retrieved."""
    namespace = "test_namespace"
    set_name = "test_set"
    user_key = "test_key_value"

    key = Key(namespace, set_name, user_key)

    assert key.namespace == namespace
    assert key.set_name == set_name
    assert key.value == user_key
    assert key.digest is not None
    assert len(key.digest) == 40  # 20 bytes = 40 hex chars

    # Verify digest is consistent
    digest1 = key.digest
    key2 = Key(namespace, set_name, user_key)
    digest2 = key2.digest
    assert digest1 == digest2

def test_key_string_representation():
    """Test string representation of keys."""
    key = Key("ns", "set", 123)

    # Test __str__
    str_repr = str(key)
    assert isinstance(str_repr, str)
    assert len(str_repr) > 0

    # Test __repr__
    repr_str = repr(key)
    assert isinstance(repr_str, str)
    assert "Key(" in repr_str

def test_key_digest_consistency():
    """Test that keys with same namespace, set, and value have same digest."""
    key1 = Key("ns", "set", "value")
    key2 = Key("ns", "set", "value")

    assert key1.digest == key2.digest
    assert key1 == key2

    # Note: Based on actual behavior, digests appear to be based on namespace, set, and value.
    # Different namespace with same set and value may have same digest (equality is digest-based).
    key3 = Key("ns2", "set", "value")
    # Keys are equal if they have the same digest, regardless of namespace/set
    assert key1.digest == key3.digest  # Same digest because same set and value
    assert key1 == key3  # Equal based on digest comparison

    # Different set should have different digest
    key4 = Key("ns", "set2", "value")
    assert key1.digest != key4.digest
    assert key1 != key4

    # Different value should have different digest
    key5 = Key("ns", "set", "value2")
    assert key1.digest != key5.digest
    assert key1 != key5

def test_key_partition_id():
    """partition_id is the digest's first two bytes, little-endian, masked to 4096 partitions."""
    keys = [Key("test", "test", i) for i in range(64)]
    for key in keys:
        digest = bytes.fromhex(key.digest)
        assert key.partition_id == int.from_bytes(digest[:2], "little") & 0x0FFF
    assert len({key.partition_id for key in keys}) > 1

def test_key_partition_id_follows_digest():
    """A key rebuilt from the digest alone lands in the same partition."""
    key = Key("test", "test", "user-1")
    assert Key.key_with_digest("other", "set", key.digest).partition_id == key.partition_id

def test_key_with_digest_invalid():
    """Test key_with_digest with invalid inputs."""
    # Invalid digest length (too short)
    with pytest.raises((ValueError, TypeError)):  # type: ignore[arg-type]
        Key.key_with_digest("ns", "set", "1234")  # Too short

    # Invalid digest length (too long)
    with pytest.raises((ValueError, TypeError)):  # type: ignore[arg-type]
        Key.key_with_digest("ns", "set", "a" * 42)  # Too long (21 bytes = 42 hex chars)

    # Invalid digest format (non-hex string)
    with pytest.raises((ValueError, TypeError)):  # type: ignore[arg-type]
        Key.key_with_digest("ns", "set", "invalid_hex_string_!!!!!!")

    # Invalid type
    with pytest.raises((ValueError, TypeError)):  # type: ignore[arg-type]
        Key.key_with_digest("ns", "set", 12345)  # Not bytes or string

    # Invalid bytes length (too short)
    with pytest.raises((ValueError, TypeError)):  # type: ignore[arg-type]
        Key.key_with_digest("ns", "set", b"short")  # Too short

    # Invalid bytes length (too long)
    with pytest.raises((ValueError, TypeError)):  # type: ignore[arg-type]
        Key.key_with_digest("ns", "set", b"x" * 21)  # Too long

def test_key_with_digest_namespace_set():
    """Test key_with_digest with different namespace and set values."""
    original_key = Key("ns1", "set1", "value")
    digest = original_key.digest

    # Same namespace and set
    key1 = Key.key_with_digest("ns1", "set1", digest)
    assert key1 == original_key
    assert key1.namespace == "ns1"
    assert key1.set_name == "set1"

    # Different namespace, same digest - keys equal (digest-based comparison)
    # but properties differ
    key2 = Key.key_with_digest("ns2", "set1", digest)
    assert key2.namespace == "ns2"
    assert key2.set_name == "set1"
    # Note: They have the same digest so they are equal
    assert key2.digest == original_key.digest

    # Different set, same digest
    key3 = Key.key_with_digest("ns1", "set2", digest)
    assert key3.namespace == "ns1"
    assert key3.set_name == "set2"

def test_key_copy():
    """Test key copying functionality."""
    original = Key("ns", "set", "value")

    # Test __copy__
    import copy
    copied = copy.copy(original)
    assert copied == original
    assert copied.namespace == original.namespace
    assert copied.set_name == original.set_name
    assert copied.value == original.value
    assert copied.digest == original.digest

    # Test __deepcopy__
    deep_copied = copy.deepcopy(original)
    assert deep_copied == original
    assert deep_copied.namespace == original.namespace
    assert deep_copied.set_name == original.set_name
    assert deep_copied.value == original.value
    assert deep_copied.digest == original.digest

def test_key_from_digest_missing_user_key():
    """Test that keys created from digest have no user key value."""
    original = Key("ns", "set", "user_key_value")
    digest = original.digest

    key_from_digest = Key.key_with_digest("ns", "set", digest)

    # Key should have no user key
    assert key_from_digest.value is None

    # But should still be equal to original (digest-based comparison)
    assert key_from_digest == original

def test_integer_key_vs_string_key():
    """Test that integer keys and string keys with same numeric value are different."""
    # Integer key
    key_int = Key("test", "test", 12345)
    assert key_int.value == 12345
    assert isinstance(key_int.value, int)
    
    # String key with same numeric value
    key_str = Key("test", "test", "12345")
    assert key_str.value == "12345"
    assert isinstance(key_str.value, str)
    
    # They should have different digests (different keys)
    assert key_int.digest != key_str.digest
    assert key_int != key_str
    
    # Verify integer keys preserve type
    key_zero = Key("test", "test", 0)
    assert key_zero.value == 0
    assert isinstance(key_zero.value, int)
    
    key_negative = Key("test", "test", -123)
    assert key_negative.value == -123
    assert isinstance(key_negative.value, int)

def test_from_user_keys_matches_one_key_at_a_time():
    """A bulk build gives the same keys, in order, as one constructor call each."""
    user_keys = [7, -3, 2**40, "alice", "", b"\x00\x01", bytearray(b"raw")]

    keys = Key.from_user_keys("test", "users", user_keys)

    assert isinstance(keys, list)
    assert [k.digest for k in keys] == [Key("test", "users", u).digest for u in user_keys]
    assert [k.value for k in keys] == [Key("test", "users", u).value for u in user_keys]
    assert all(k.namespace == "test" and k.set_name == "users" for k in keys)

def test_from_user_keys_accepts_any_iterable():
    """Tuples, ranges and generators work as well as lists; nothing in gives nothing out."""
    expected = [Key("test", "users", i).digest for i in range(3)]

    assert [k.digest for k in Key.from_user_keys("test", "users", (0, 1, 2))] == expected
    assert [k.digest for k in Key.from_user_keys("test", "users", range(3))] == expected
    assert [k.digest for k in Key.from_user_keys("test", "users", (i for i in range(3)))] == expected
    assert Key.from_user_keys("test", "users", []) == []

def test_from_user_keys_rejects_a_value_that_cannot_be_a_key():
    """A float is not a key type, wherever it sits in the input."""
    with pytest.raises(ValueError):
        Key.from_user_keys("test", "users", [1, 2.5])

def test_from_user_keys_rejects_a_non_iterable():
    with pytest.raises(TypeError):
        Key.from_user_keys("test", "users", 5)
