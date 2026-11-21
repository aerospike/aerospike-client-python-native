#!/usr/bin/env python3
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

"""
Fix generated .pyi only where stub_gen can't. Prefer .rs annotations and stub_gen;
add postprocess logic only when stub_gen can't produce the stub.

What remains here, and why stub_gen cannot do it:

1. Exceptions: ``create_exception!`` types and the Python-defined ``ServerError``
   hierarchy never pass through stub_gen, so the exceptions submodule stub and
   its runtime ``__init__.py`` are written here.
2. ``enum.IntEnum`` bases for the ``eq_int`` flag enums, which behave as ints at
   runtime but are emitted as plain ``enum.Enum``.
3. Dunder and literal fix-ups stub_gen cannot infer: ``__richcmp__`` -> ``__eq__``
   / ``__ne__``, self-referential classattr values, the ``ListPolicy.write_flags``
   bitmask type.
4. ``__all__`` and the package ``__init__.pyi`` re-export shim.

Everything else is driven by ``#[gen_stub_pyclass]`` / ``#[gen_stub_pymethods]``
in the Rust sources. All fixes are idempotent - safe to run multiple times.
"""

import os
import sys
import re


# Exception class stub definitions (defined once, reused everywhere)
EXCEPTION_STUB_CLASSES = '''class AerospikeError(builtins.Exception):
    """Base exception class for all Aerospike-specific errors."""
    @property
    def result_code(self) -> typing.Optional[ResultCode]: ...
    @property
    def in_doubt(self) -> builtins.bool: ...
    @property
    def node(self) -> typing.Optional[builtins.str]: ...
    @property
    def iteration(self) -> typing.Optional[builtins.int]: ...
    @property
    def base_message(self) -> typing.Optional[builtins.str]: ...
    @property
    def sub_exceptions(self) -> typing.Optional[builtins.list]: ...
    def __init__(self, message: builtins.str) -> None: ...

class ServerError(AerospikeError):
    """Exception raised when the Aerospike server returns an error."""
    def __init__(self, message: builtins.str, result_code: ResultCode, in_doubt: builtins.bool = False, sub_code: typing.Optional[builtins.int] = None, server_message: typing.Optional[builtins.str] = None, exp_trace: typing.Optional[typing.Any] = None, node: typing.Optional[builtins.str] = None, iteration: typing.Optional[builtins.int] = None, base_message: typing.Optional[builtins.str] = None, sub_exceptions: typing.Optional[builtins.list] = None) -> None: ...
    @property
    def result_code(self) -> ResultCode: ...
    @property
    def in_doubt(self) -> builtins.bool: ...
    @property
    def sub_code(self) -> typing.Optional[builtins.int]: ...
    @property
    def server_message(self) -> typing.Optional[builtins.str]: ...
    @property
    def exp_trace(self) -> typing.Optional[typing.Any]: ...
    @property
    def node(self) -> typing.Optional[builtins.str]: ...
    @property
    def iteration(self) -> typing.Optional[builtins.int]: ...
    @property
    def base_message(self) -> typing.Optional[builtins.str]: ...
    @property
    def sub_exceptions(self) -> typing.Optional[builtins.list]: ...

class UDFBadResponse(AerospikeError):
    """Exception raised when a UDF (User Defined Function) returns a bad response."""
    def __init__(self, message: builtins.str) -> None: ...

class TimeoutError(AerospikeError):
    """Exception raised when an operation times out."""
    def __init__(self, message: builtins.str) -> None: ...

class Base64DecodeError(AerospikeError):
    """Exception raised when Base64 decoding fails."""
    def __init__(self, message: builtins.str) -> None: ...

class InvalidUTF8(AerospikeError):
    """Exception raised when invalid UTF-8 is encountered."""
    def __init__(self, message: builtins.str) -> None: ...

class IoError(AerospikeError):
    """Exception raised for I/O related errors."""
    def __init__(self, message: builtins.str) -> None: ...

class ParseAddressError(AerospikeError):
    """Exception raised when parsing an address fails."""
    def __init__(self, message: builtins.str) -> None: ...

class ParseIntError(AerospikeError):
    """Exception raised when parsing an integer fails."""
    def __init__(self, message: builtins.str) -> None: ...

class ConnectionError(AerospikeError):
    """Exception raised when a connection error occurs."""
    def __init__(self, message: builtins.str) -> None: ...

class ValueError(AerospikeError):
    """Exception raised when an invalid value is provided."""
    def __init__(self, message: builtins.str) -> None: ...

class RecvError(AerospikeError):
    """Exception raised when receiving data fails."""
    def __init__(self, message: builtins.str) -> None: ...

class PasswordHashError(AerospikeError):
    """Exception raised when password hashing fails."""
    def __init__(self, message: builtins.str) -> None: ...

class BadResponse(AerospikeError):
    """Exception raised when a bad response is received."""
    def __init__(self, message: builtins.str) -> None: ...

class InvalidRustClientArgs(AerospikeError):
    """Exception raised when invalid arguments are provided to the Rust client."""
    def __init__(self, message: builtins.str) -> None: ...

class InvalidNodeError(AerospikeError):
    """Exception raised when an invalid node is encountered."""
    def __init__(self, message: builtins.str) -> None: ...

class InvalidNamespaceError(AerospikeError):
    """Exception raised when an invalid namespace is encountered."""
    def __init__(self, message: builtins.str) -> None: ...

class NoMoreConnections(AerospikeError):
    """Exception raised when no more connections are available."""
    def __init__(self, message: builtins.str) -> None: ...

class ClientError(AerospikeError):
    """Exception raised for client-side errors."""
    def __init__(self, message: builtins.str) -> None: ...

class CommitFailedError(AerospikeError):
    """Exception raised when a multi-record transaction commit fails."""
    def __init__(self, message: builtins.str) -> None: ...

class BatchFailedError(ClientError):
    """A batch command failed as a whole.

    ``records`` carries the per-key ``BatchRecord`` outcomes attached to the
    failure: rows the server answered keep their result, unanswered rows
    carry the stamped result code (``TIMEOUT`` on client timeouts) and the
    per-row in-doubt flag.
    """
    records: typing.Optional[builtins.list[BatchRecord]]
    def __init__(self, message: builtins.str) -> None: ...

class MaxErrorRate(AerospikeError):
    """Per-node circuit breaker tripped (client-side, not sent to server)."""
    def __init__(self, message: builtins.str) -> None: ...
'''

# ServerError subclasses (Python-defined; used by Rust create_server_error dispatch)
SERVER_ERROR_SUBCLASS_STUBS = '''
class RecordError(ServerError):
    """Record-level server errors."""
    def __init__(self, message: builtins.str, result_code: ResultCode, in_doubt: builtins.bool = False) -> None: ...

class IndexError(ServerError):
    """Index-related server errors."""
    def __init__(self, message: builtins.str, result_code: ResultCode, in_doubt: builtins.bool = False) -> None: ...

class SecurityError(ServerError):
    """Security and authentication server errors."""
    def __init__(self, message: builtins.str, result_code: ResultCode, in_doubt: builtins.bool = False) -> None: ...

class QueryError(ServerError):
    """Query and scan server errors."""
    def __init__(self, message: builtins.str, result_code: ResultCode, in_doubt: builtins.bool = False) -> None: ...

class BatchError(ServerError):
    """Batch subsystem server errors."""
    def __init__(self, message: builtins.str, result_code: ResultCode, in_doubt: builtins.bool = False) -> None: ...

class QuotaError(ServerError):
    """Quota server errors."""
    def __init__(self, message: builtins.str, result_code: ResultCode, in_doubt: builtins.bool = False) -> None: ...

class UdfError(ServerError):
    """Server-side UDF execution errors."""
    def __init__(self, message: builtins.str, result_code: ResultCode, in_doubt: builtins.bool = False) -> None: ...

class RecordNotFound(RecordError): ...
class GenerationError(RecordError): ...
class InvalidRequest(ServerError): ...
class RecordExistsError(RecordError): ...
class BinTypeError(RecordError): ...
class RecordTooBig(RecordError): ...
class BinNotFound(RecordError): ...
class FilteredOut(ServerError): ...
class OpNotApplicable(ServerError): ...
class IndexNotFound(IndexError): ...
class IndexFoundError(IndexError): ...
class NotAuthenticated(SecurityError): ...
class SecurityNotEnabled(SecurityError): ...
'''

EXCEPTION_STUB__ALL__ = '''
__all__ = [
    "AerospikeError",
    "ServerError",
    "UDFBadResponse",
    "TimeoutError",
    "BadResponse",
    "ConnectionError",
    "InvalidNodeError",
    "InvalidNamespaceError",
    "NoMoreConnections",
    "CommitFailedError",
    "BatchFailedError",
    "RecvError",
    "Base64DecodeError",
    "InvalidUTF8",
    "ParseAddressError",
    "ParseIntError",
    "ValueError",
    "IoError",
    "PasswordHashError",
    "InvalidRustClientArgs",
    "ClientError",
    "MaxErrorRate",
    "ResultCode",
    "RecordError",
    "IndexError",
    "SecurityError",
    "RecordNotFound",
    "GenerationError",
    "InvalidRequest",
    "RecordExistsError",
    "BinTypeError",
    "RecordTooBig",
    "BinNotFound",
    "FilteredOut",
    "OpNotApplicable",
    "IndexNotFound",
    "IndexFoundError",
    "NotAuthenticated",
    "SecurityNotEnabled",
    "QueryError",
    "BatchError",
    "QuotaError",
    "UdfError",
]
'''

EXCEPTIONS_SUBMODULE_STUB = f'''# This file contains type stubs for the aerospike_native.exceptions submodule
# Generated by postprocess_stubs.py

import builtins
import typing
from .._native import BatchRecord, ResultCode

# Exception classes
{EXCEPTION_STUB_CLASSES}
{SERVER_ERROR_SUBCLASS_STUBS}

# ResultCode is re-exported from the main module for convenience
# It's defined in _native and added to exceptions submodule at runtime
{EXCEPTION_STUB__ALL__}'''


def fix_imports(content: str, pyi_file_path: str = "") -> str:
    """Fix import statements to use relative imports and avoid circular dependencies."""
    # Fix absolute imports from aerospike_native._native to relative imports
    content = re.sub(
        r'from aerospike_native\._native import ([\w, ]+)',
        r'from ._native import \1',
        content
    )
    # Fix any existing circular imports
    content = re.sub(
        r'^from aerospike_native import (Key|Record|Blob|GeoJSON|HLL|List|Map|Client)\b',
        r'from ._native import \1',
        content,
        flags=re.MULTILINE
    )

    # Remove self-imports in _native.pyi - types are now all in the same module
    if '_native.pyi' in pyi_file_path:
        content = re.sub(
            r'^from _native import [^\n]+\n',
            '',
            content,
            flags=re.MULTILINE
        )
        # pyo3_stub_gen emits a bare "import aerospike_native" at the top of the
        # native submodule stub. It's unused (the file only references local
        # types) and triggers "unused import" warnings in IDEs.
        content = re.sub(
            r'^import aerospike_native\s*\n',
            '',
            content,
            count=1,
            flags=re.MULTILINE,
        )

    return content


def fix_list_policy_write_flags_type(content: str) -> str:
    """Narrow ListPolicy write_flags from Any to Union[ListWriteFlags, int] for better type hints."""
    content = re.sub(
        r'def __new__\(cls, order:typing\.Optional\[ListOrderType\]=None, '
        r'write_flags:typing\.Optional\[typing\.Any\]=None\) -> ListPolicy:',
        'def __new__(cls, order: typing.Optional[ListOrderType] = None, '
        'write_flags: typing.Optional[typing.Union[ListWriteFlags, int]] = None) -> ListPolicy:',
        content,
    )
    n = 0
    content, c = re.subn(
        r'(@write_flags\.setter\s+def write_flags\(self, value:) typing\.Any(\) -> None:)',
        r'\1 typing.Union[ListWriteFlags, int]\2',
        content,
        count=1,
    )
    n += c
    if n:
        print('  ✓ Narrowed ListPolicy.write_flags type to Union[ListWriteFlags, int]')
    content, c2 = re.subn(
        r'(def write_flags\(self\)\s*->\s*)ListWriteFlags(\s*:)',
        r'\1builtins.int\2',
        content,
        count=1,
    )
    if c2:
        print('  ✓ ListPolicy.write_flags getter stub uses int (bitmask)')
    return content


def fix_map_write_flags_int_enum(content: str) -> str:
    """Use IntEnum for PNC flag enums so int() and bitmask ops type-check.

    pyo3_stub_gen emits ``(enum.Enum)`` for all pyclass enums; several are ``eq_int``
    in Rust and behave as integers at runtime. MapWriteFlags was the original
    case; Exp*/HLL write/read flags need the same stub treatment.

    Also rewrites each member line from ``NAME = ...`` to ``NAME: int`` within
    every ``class X(enum.IntEnum):`` block. pyo3_stub_gen emits members with an
    ellipsis value; for IntEnum subclasses Pyright/PyCharm flag that as
    ``EllipsisType is not assignable to int``. The typeshed convention for
    IntEnum stubs is an annotation-only form (e.g. ``SIGABRT: int``).
    """
    for cls_name in (
        "MapWriteFlags",
        "ExpReadFlags",
        "ExpWriteFlags",
        "HLLWriteFlags",
        "BitWriteFlags",
        "ListWriteFlags",
    ):
        content = re.sub(
            rf'^class {cls_name}\((?:enum\.)?Enum\):',
            f'class {cls_name}(enum.IntEnum):',
            content,
            count=1,
            flags=re.MULTILINE,
        )

    def _rewrite_int_enum_block(match: re.Match) -> str:
        header = match.group(1)
        body = match.group(2)
        new_body, n = re.subn(
            r'^(    )([A-Z_][A-Z0-9_]*) = \.\.\.(\s*)$',
            r'\1\2: builtins.int\3',
            body,
            flags=re.MULTILINE,
        )
        _rewrite_int_enum_block.total += n  # type: ignore[attr-defined]
        return header + new_body

    _rewrite_int_enum_block.total = 0  # type: ignore[attr-defined]
    content = re.sub(
        r'(^class \w+\((?:enum\.)?IntEnum\):\n)((?:(?:    .*)?\n)*?)(?=^\S|\Z)',
        _rewrite_int_enum_block,
        content,
        flags=re.MULTILINE,
    )
    total = _rewrite_int_enum_block.total  # type: ignore[attr-defined]
    if total:
        print(f"  ✓ Rewrote {total} IntEnum member(s) to annotation-only form")
    return content


def fix_circular_classattr_refs(content: str) -> str:
    """Replace pyo3_stub_gen circular self-reference values with ``...``.

    When a class with unique constant values gets ``#[gen_stub_pymethods]``,
    pyo3_stub_gen emits self-referential values like::

        MAP_KEY: _native.LoopVarPart = LoopVarPart.MAP_KEY

    These circular references confuse type checkers.  Replace every occurrence
    of ``ATTR_NAME: [module.]ClassName = ClassName.ATTR_NAME`` with
    ``ATTR_NAME: [module.]ClassName = ...``.
    """
    fixed, n = re.subn(
        r'^( {4})([A-Z_]+): ((?:\w+\.)?(\w+)) = \4\.\2\s*$',
        r'\1\2: \3 = ...',
        content,
        flags=re.MULTILINE,
    )
    if n:
        print(f"  ✓ Fixed {n} circular classattr self-reference(s) (pyo3_stub_gen limitation)")
    return fixed


def fix_richcmp_stubs(content: str) -> str:
    """Replace ``__richcmp__`` stubs with the ``__eq__`` / ``__ne__`` Python sees.

    ``__richcmp__`` is the pyo3 slot behind every comparison operator; stub
    generation emits it verbatim, but no Python caller can name it. The
    classes here only implement equality, so declare that pair.
    """
    fixed, n = re.subn(
        r'^( {4})def __richcmp__\(self, other: [^,]+, op: int\) -> builtins\.bool: \.\.\.\s*$',
        r'\1def __eq__(self, other: object) -> builtins.bool: ...\n'
        r'\1def __ne__(self, other: object) -> builtins.bool: ...',
        content,
        flags=re.MULTILINE,
    )
    if n:
        print(f"  ✓ Rewrote {n} __richcmp__ stub(s) as __eq__/__ne__")
    return fixed


def ensure_exports(content: str) -> str:
    """Ensure all classes are properly imported from _native in __init__.pyi.

    The runtime __init__.py uses 'from ._native import *', so the stub
    should match this for consistency. We also explicitly re-export Key, Client, and Record
    for better type checker support.
    """
    # Use 'import *' to match runtime behavior - all classes are available
    if 'from ._native import *' not in content:
        # Replace any existing specific imports with wildcard import
        content = re.sub(
            r'from \._native import [^\n]+\n',
            'from ._native import *\n',
            content
        )
        # Or add it if no import exists
        if 'from ._native' not in content:
            if 'import builtins' in content:
                content = re.sub(
                    r'(import builtins\n)',
                    r'\1from ._native import *\n',
                    content
                )
            else:
                # Add at the beginning after any existing imports
                content = 'from ._native import *\n' + content

    # Explicitly re-export commonly used classes for type checker clarity
    # Some type checkers don't fully resolve 'import *', so explicit re-exports help
    needed_exports = ['Key', 'Client', 'Record', 'ReadPolicy', 'WritePolicy', 'FilterExpression', 'BasePolicy']
    missing_exports = []
    for export in needed_exports:
        if not re.search(rf'^{export}\s*:\s*type\s*=\s*_native\.{export}\b', content, re.MULTILINE):
            missing_exports.append(export)

    if missing_exports:
        class_match = re.search(r'^(class\s+\w+)', content, re.MULTILINE)
        if class_match:
            insert_pos = class_match.start()
            # Build re-exports - keep existing ones, add missing ones
            existing_lines = []
            new_lines = []
            for export in needed_exports:
                if export not in missing_exports:
                    # Extract existing line
                    existing_match = re.search(rf'^{export}\s*:\s*type\s*=\s*_native\.{export}\b', content, re.MULTILINE)
                    if existing_match:
                        existing_lines.append((existing_match.start(), existing_match.end()))
                else:
                    new_lines.append(f'{export}: type = _native.{export}')

            # Remove existing re-export section if it exists, then add complete one
            re_export_match = re.search(r'^# (Re-export|Explicit re-exports)', content, re.MULTILINE)
            if re_export_match:
                # Find the end of the re-export section (before next class or blank line)
                section_start = re_export_match.start()
                next_section = re.search(r'\n\n(?=class\s|\n#)', content[section_start:], re.MULTILINE)
                if next_section:
                    section_end = section_start + next_section.start() + 1
                    content = content[:section_start] + content[section_end:]
                    # Re-find class position after removal
                    class_match = re.search(r'^(class\s+\w+)', content, re.MULTILINE)
                    if class_match:
                        insert_pos = class_match.start()

            # Add all re-exports (both existing and new)
            all_exports = [f'{exp}: type = _native.{exp}' for exp in needed_exports]
            re_exports = '# Explicit re-exports for type checking\n' + '\n'.join(all_exports) + '\n\n'
            content = content[:insert_pos] + re_exports + content[insert_pos:]

    return content


HOST_LIKE_PROTOCOL = '''class _HostLike(typing.Protocol):
    r"""
    Any object with a host's name and port, such as the host class of a
    layer above. The seed candidate providers accept it in place of
    :class:`Host`, and also read a ``tls_name`` attribute when it has one.
    """
    @property
    def name(self) -> builtins.str: ...
    @property
    def port(self) -> builtins.int: ...

'''


def add_host_like_protocol(content: str) -> str:
    """Define the stub-only ``_HostLike`` protocol the provider signatures name.

    It has no runtime counterpart, so it must be added after ``__all__`` is
    built: ``from ._native import *`` would otherwise promise a name the
    extension does not have.
    """
    first_class = re.search(r'^class ', content, re.MULTILINE)
    if '_HostLike' not in content or 'class _HostLike(' in content or not first_class:
        return content
    print("  ✓ Added the _HostLike protocol")
    return content[:first_class.start()] + HOST_LIKE_PROTOCOL + content[first_class.start():]


def add_dunder_all(content: str) -> str:
    """Generate __all__ from top-level class/function definitions so `import *` is resolved by all type checkers."""
    names = sorted(set(
        re.findall(r'^class (\w+)[\s(:]', content, re.MULTILINE)
        + re.findall(r'^def (\w+)\(', content, re.MULTILINE)
    ))
    if not names:
        return content
    all_block = '__all__ = [\n' + ''.join(f'    "{n}",\n' for n in names) + ']\n'
    if '__all__' in content:
        content = re.sub(r'^__all__\s*=\s*\[.*?\]\s*\n', all_block, content, count=1, flags=re.DOTALL | re.MULTILINE)
        print(f'  ✓ Replaced __all__ ({len(names)} names)')
    else:
        first_class = re.search(r'^class ', content, re.MULTILINE)
        if first_class:
            content = content[:first_class.start()] + all_block + '\n' + content[first_class.start():]
        else:
            content = all_block + '\n' + content
        print(f'  ✓ Added __all__ ({len(names)} names)')
    return content


def ensure_exceptions_submodule(package_dir: str):
    """Always regenerate the exceptions submodule stub file and runtime __init__.py."""
    exceptions_dir = os.path.join(package_dir, 'exceptions')
    os.makedirs(exceptions_dir, exist_ok=True)

    # Always write stub file for type checking (regenerate every time)
    init_stub_path = os.path.join(exceptions_dir, '__init__.pyi')
    with open(init_stub_path, 'w') as f:
        f.write(EXCEPTIONS_SUBMODULE_STUB)
    print(f"  ✓ Regenerated exceptions submodule stub: {init_stub_path}")

    # Always write runtime __init__.py (regenerate every time)
    # PyO3's create_exception! creates exceptions in aerospike_native.exceptions
    # The exceptions submodule is created by PyO3 when we call add_submodule
    # We need to access it from the parent package
    init_py_path = os.path.join(exceptions_dir, '__init__.py')
    with open(init_py_path, 'w') as f:
        f.write('# Exceptions are created by PyO3 in this submodule\n')
        f.write('# via create_exception!(aerospike_native.exceptions, ...) and add_submodule\n')
        f.write('# Users can import: from aerospike_native.exceptions import AerospikeError\n')
        f.write('\n')
        f.write('from .. import _native\n')
        f.write('\n')
        f.write('# Access the exceptions submodule created by PyO3\n')
        f.write('_exceptions = getattr(_native, "exceptions", None)\n')
        f.write('if _exceptions is None:\n')
        f.write('    raise ImportError("Exceptions submodule not found in native module")\n')
        f.write('\n')
        f.write('# Re-export all exception classes\n')
        f.write('AerospikeError = _exceptions.AerospikeError\n')
        f.write('ServerError = _exceptions.ServerError\n')
        f.write('UDFBadResponse = _exceptions.UDFBadResponse\n')
        f.write('TimeoutError = _exceptions.TimeoutError\n')
        f.write('BadResponse = _exceptions.BadResponse\n')
        f.write('ConnectionError = _exceptions.ConnectionError\n')
        f.write('InvalidNodeError = _exceptions.InvalidNodeError\n')
        f.write('InvalidNamespaceError = _exceptions.InvalidNamespaceError\n')
        f.write('NoMoreConnections = _exceptions.NoMoreConnections\n')
        f.write('CommitFailedError = _exceptions.CommitFailedError\n')
        f.write('BatchFailedError = _exceptions.BatchFailedError\n')
        f.write('RecvError = _exceptions.RecvError\n')
        f.write('Base64DecodeError = _exceptions.Base64DecodeError\n')
        f.write('InvalidUTF8 = _exceptions.InvalidUTF8\n')
        f.write('ParseAddressError = _exceptions.ParseAddressError\n')
        f.write('ParseIntError = _exceptions.ParseIntError\n')
        f.write('ValueError = _exceptions.ValueError\n')
        f.write('IoError = _exceptions.IoError\n')
        f.write('PasswordHashError = _exceptions.PasswordHashError\n')
        f.write('InvalidRustClientArgs = _exceptions.InvalidRustClientArgs\n')
        f.write('ClientError = _exceptions.ClientError\n')
        f.write('MaxErrorRate = _exceptions.MaxErrorRate\n')
        f.write('# ResultCode is in the main native module, not in exceptions submodule\n')
        f.write('ResultCode = _native.ResultCode\n')
        f.write('\n')
        f.write('# The result code lives on the base so every error answers it: the\n')
        f.write('# native layer sets the instance attribute on each failure it raises,\n')
        f.write('# server and client side alike, so only a hand-built instance is None.\n')
        f.write('AerospikeError.result_code = None\n')
        f.write('# Typed in-doubt lives on the base so every error answers it; the native\n')
        f.write('# layer sets the instance attribute only when core reports the write may\n')
        f.write('# have landed.\n')
        f.write('AerospikeError.in_doubt = False\n')
        f.write('# Retry/diagnostic context defaults, same mechanism: the native layer\n')
        f.write('# sets the instance attribute only when the retry loop recorded a value.\n')
        f.write('AerospikeError.node = None\n')
        f.write('AerospikeError.iteration = None\n')
        f.write('AerospikeError.base_message = None\n')
        f.write('AerospikeError.sub_exceptions = None\n')
        f.write('# Per-key outcomes; the native layer attaches the list on batch failures.\n')
        f.write('BatchFailedError.records = None\n')
        f.write('\n')
        f.write('# ServerError subclasses for specific result codes (grouping bases first)\n')
        f.write('class RecordError(ServerError):\n')
        f.write('    """Record-level server errors."""\n\n\n')
        f.write('class IndexError(ServerError):\n')
        f.write('    """Index-related server errors."""\n\n\n')
        f.write('class SecurityError(ServerError):\n')
        f.write('    """Security and authentication server errors."""\n\n\n')
        f.write('# Tier 1 — core server errors\n')
        f.write('class RecordNotFound(RecordError):\n')
        f.write('    """Record not found (KEY_NOT_FOUND_ERROR)."""\n\n')
        f.write('class GenerationError(RecordError):\n')
        f.write('    """Generation check failed (GENERATION_ERROR)."""\n\n')
        f.write('class InvalidRequest(ServerError):\n')
        f.write('    """Invalid request / parameter error (PARAMETER_ERROR)."""\n\n')
        f.write('class RecordExistsError(RecordError):\n')
        f.write('    """Record already exists (KEY_EXISTS_ERROR)."""\n\n')
        f.write('class BinTypeError(RecordError):\n')
        f.write('    """Bin type incompatible (BIN_TYPE_ERROR)."""\n\n')
        f.write('class RecordTooBig(RecordError):\n')
        f.write('    """Record too big (RECORD_TOO_BIG)."""\n\n')
        f.write('class BinNotFound(RecordError):\n')
        f.write('    """Bin not found (BIN_NOT_FOUND)."""\n\n')
        f.write('class FilteredOut(ServerError):\n')
        f.write('    """Record filtered out (FILTERED_OUT)."""\n\n')
        f.write('class OpNotApplicable(ServerError):\n')
        f.write('    """Operation not applicable (OP_NOT_APPLICABLE)."""\n\n')
        f.write('# Tier 2 — index and security\n')
        f.write('class IndexNotFound(IndexError):\n')
        f.write('    """Index not found (INDEX_NOT_FOUND)."""\n\n')
        f.write('class IndexFoundError(IndexError):\n')
        f.write('    """Index already exists (INDEX_FOUND)."""\n\n')
        f.write('class NotAuthenticated(SecurityError):\n')
        f.write('    """Not authenticated (NOT_AUTHENTICATED)."""\n\n')
        f.write('class SecurityNotEnabled(SecurityError):\n')
        f.write('    """Security not enabled (SECURITY_NOT_ENABLED)."""\n\n')
        f.write('# Tier 3 — subsystem families (query/scan, batch, quota, UDF)\n')
        f.write('class QueryError(ServerError):\n')
        f.write('    """Query and scan server errors."""\n\n')
        f.write('class BatchError(ServerError):\n')
        f.write('    """Batch subsystem server errors."""\n\n')
        f.write('class QuotaError(ServerError):\n')
        f.write('    """Quota server errors."""\n\n')
        f.write('class UdfError(ServerError):\n')
        f.write('    """Server-side UDF execution errors (UDF_BAD_RESPONSE)."""\n\n')
        f.write('# ResultCode -> exception class for Rust create_server_error() dispatch\n')
        f.write('_RC_TO_CLS = {\n')
        f.write('    ResultCode.KEY_NOT_FOUND_ERROR: RecordNotFound,\n')
        f.write('    ResultCode.GENERATION_ERROR: GenerationError,\n')
        f.write('    ResultCode.PARAMETER_ERROR: InvalidRequest,\n')
        f.write('    ResultCode.KEY_EXISTS_ERROR: RecordExistsError,\n')
        f.write('    ResultCode.BIN_TYPE_ERROR: BinTypeError,\n')
        f.write('    ResultCode.RECORD_TOO_BIG: RecordTooBig,\n')
        f.write('    ResultCode.BIN_NOT_FOUND: BinNotFound,\n')
        f.write('    ResultCode.FILTERED_OUT: FilteredOut,\n')
        f.write('    ResultCode.OP_NOT_APPLICABLE: OpNotApplicable,\n')
        f.write('    ResultCode.INDEX_NOT_FOUND: IndexNotFound,\n')
        f.write('    ResultCode.INDEX_FOUND: IndexFoundError,\n')
        f.write('    ResultCode.NOT_AUTHENTICATED: NotAuthenticated,\n')
        f.write('    ResultCode.SECURITY_NOT_ENABLED: SecurityNotEnabled,\n')
        f.write('    # Security family (flat: the finer authentication-vs-\n')
        f.write('    # authorization split is an SDK-level concern)\n')
        f.write('    ResultCode.ILLEGAL_STATE: SecurityError,\n')
        f.write('    ResultCode.INVALID_USER: SecurityError,\n')
        f.write('    ResultCode.USER_ALREADY_EXISTS: SecurityError,\n')
        f.write('    ResultCode.INVALID_PASSWORD: SecurityError,\n')
        f.write('    ResultCode.EXPIRED_PASSWORD: SecurityError,\n')
        f.write('    ResultCode.FORBIDDEN_PASSWORD: SecurityError,\n')
        f.write('    ResultCode.INVALID_CREDENTIAL: SecurityError,\n')
        f.write('    ResultCode.EXPIRED_SESSION: SecurityError,\n')
        f.write('    ResultCode.INVALID_ROLE: SecurityError,\n')
        f.write('    ResultCode.ROLE_ALREADY_EXISTS: SecurityError,\n')
        f.write('    ResultCode.INVALID_PRIVILEGE: SecurityError,\n')
        f.write('    ResultCode.INVALID_ALLOWLIST: SecurityError,\n')
        f.write('    ResultCode.ROLE_VIOLATION: SecurityError,\n')
        f.write('    ResultCode.NOT_ALLOWLISTED: SecurityError,\n')
        f.write('    ResultCode.SECURITY_NOT_SUPPORTED: SecurityError,\n')
        f.write('    ResultCode.SECURITY_SCHEME_NOT_SUPPORTED: SecurityError,\n')
        f.write('    # Query/scan family\n')
        f.write('    ResultCode.QUERY_GENERIC: QueryError,\n')
        f.write('    ResultCode.QUERY_ABORTED: QueryError,\n')
        f.write('    ResultCode.QUERY_QUEUE_FULL: QueryError,\n')
        f.write('    ResultCode.QUERY_NETIO_ERR: QueryError,\n')
        f.write('    ResultCode.QUERY_DUPLICATE: QueryError,\n')
        f.write('    ResultCode.SCAN_ABORT: QueryError,\n')
        f.write('    # Batch family\n')
        f.write('    ResultCode.BATCH_DISABLED: BatchError,\n')
        f.write('    ResultCode.BATCH_MAX_REQUESTS_EXCEEDED: BatchError,\n')
        f.write('    ResultCode.BATCH_QUEUES_FULL: BatchError,\n')
        f.write('    # Quota family\n')
        f.write('    ResultCode.QUOTA_EXCEEDED: QuotaError,\n')
        f.write('    ResultCode.QUOTAS_NOT_ENABLED: QuotaError,\n')
        f.write('    ResultCode.INVALID_QUOTA: QuotaError,\n')
        f.write('    # UDF\n')
        f.write('    ResultCode.UDF_BAD_RESPONSE: UdfError,\n')
        f.write('}\n\n')
        f.write('def _get_server_error_class(result_code):\n')
        f.write('    """Return the ServerError subclass for the given result code, or ServerError."""\n')
        f.write('    return _RC_TO_CLS.get(result_code, ServerError)\n')
    print(f"  ✓ Regenerated exceptions submodule runtime __init__.py: {init_py_path}")


def postprocess_stubs(pyi_file_path: str):
    """Post-process the generated .pyi file to fix all stub issues."""
    print(f"Post-processing stubs: {pyi_file_path}")

    with open(pyi_file_path, 'r') as f:
        content = f.read()

    content = fix_imports(content, pyi_file_path)
    package_dir = os.path.dirname(pyi_file_path)

    if '__init__.pyi' in pyi_file_path:
        # Process main package __init__.pyi
        content = ensure_exports(content)
        # Exceptions are only available via aerospike_native.exceptions submodule
        # (handled by ensure_exceptions_submodule)
        ensure_exceptions_submodule(package_dir)

    elif '_native.pyi' in pyi_file_path:
        # Process native module stub
        content = fix_list_policy_write_flags_type(content)
        content = fix_map_write_flags_int_enum(content)
        content = fix_circular_classattr_refs(content)
        content = fix_richcmp_stubs(content)
        content = add_dunder_all(content)
        content = add_host_like_protocol(content)

        # When processing native module, ensure package structure exists
        # Always create/regenerate main package __init__.pyi
        package_init_path = os.path.join(package_dir, '__init__.pyi')
        with open(package_init_path, 'w') as f:
            f.write('# This file is automatically generated by postprocess_stubs.py\n# ruff: noqa: E501\n\nfrom ._native import *\n')
        print(f"  ✓ Regenerated package __init__.pyi: {package_init_path}")
        # Recursively process the new file
        postprocess_stubs(package_init_path)

        ensure_exceptions_submodule(package_dir)

    # Clean up blank lines - remove trailing whitespace from blank lines
    lines = content.split('\n')
    cleaned_lines = [line.rstrip() if line.strip() == '' else line for line in lines]
    content = '\n'.join(cleaned_lines)

    # Ensure file ends with a newline
    if not content.endswith('\n'):
        content += '\n'

    with open(pyi_file_path, 'w') as f:
        f.write(content)

    print(f"✓ Completed post-processing: {pyi_file_path}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python postprocess_stubs.py <path_to_pyi_file>")
        sys.exit(1)

    pyi_file = sys.argv[1]
    if not os.path.exists(pyi_file):
        print(f"Error: File {pyi_file} does not exist")
        sys.exit(1)

    postprocess_stubs(pyi_file)
