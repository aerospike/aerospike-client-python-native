// Copyright 2023-2026 Aerospike, Inc.
//
// Portions may be licensed to Aerospike, Inc. under one or more contributor
// license agreements WHICH ARE COMPATIBLE WITH THE APACHE LICENSE, VERSION 2.0.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations under
// the License.

use std::fmt;

use indexmap::IndexMap;
use pyo3::prelude::*;

use pyo3_async_runtimes::tokio as pyo3_asyncio;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};



use crate::blocking::run_blocking;
use crate::errors::RustClientError;
use crate::policies::AdminPolicy;

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  PrivilegeCode
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Privilege code for access control.
    #[gen_stub_pyclass_enum(module = "_native")]
    #[pyclass(from_py_object, module = "_native")]
    #[derive(Debug, Clone, Copy)]
    pub enum PrivilegeCode {
        /// User can edit/remove other users.  Global scope only.
        UserAdmin,

        /// User can perform systems administration functions on a database that do not involve user
        /// administration.  Examples include server configuration.
        /// Global scope only.
        SysAdmin,

        /// User can perform UDF and SINDEX administration actions. Global scope only.
        DataAdmin,

        /// User can perform user defined function(UDF) administration actions.
        /// Examples include create/drop UDF. Global scope only.
        /// Requires server version 6+
        UDFAdmin,

        /// User can perform secondary index administration actions.
        /// Examples include create/drop index. Global scope only.
        /// Requires server version 6+
        SIndexAdmin,

        /// User can read data only.
        Read,

        /// User can read and write data.
        ReadWrite,

        /// User can read and write data through user defined functions.
        ReadWriteUDF,

        /// User can read and write data through user defined functions.
        Write,

        /// User can truncate data only.
        /// Requires server version 6+
        Truncate,

        /// User can perform data masking administration actions.
        /// Global scope only.
        MaskingAdmin,

        /// User can read masked data only.
        ReadMasked,

        /// User can write masked data only.
        WriteMasked,

        /// A privilege the server reported that this client has no name for.
        /// Only ever read back from a role listing; the raw code shows in the
        /// ``Privilege`` string as ``unknown-<code>``. Granting it raises
        /// :class:`ValueError`.
        Unknown,
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Version
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass)]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Clone)]
    pub struct Version {
        pub(crate) _as: aerospike_core::Version,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Version {
        #[getter]
        pub fn major(&self) -> u64 {
            self._as.major
        }

        #[getter]
        pub fn minor(&self) -> u64 {
            self._as.minor
        }

        #[getter]
        pub fn patch(&self) -> u64 {
            self._as.patch
        }

        #[getter]
        pub fn build(&self) -> u64 {
            self._as.build
        }

        /// Returns true if server supports extended read ops (CDT,
        /// expression, bit, HLL reads) in foreground query ops projection.
        /// Earlier servers only accept basic ``Read`` ops attached to a
        /// query statement. Requires server >= 8.1.2.
        pub fn supports_query_ops_projection_ext(&self) -> bool {
            self._as.supports_query_ops_projection_ext()
        }

        /// Returns true if server supports Multi-Record Transactions
        /// (MRT). Requires server >= 8.0.0.
        pub fn supports_mrt(&self) -> bool {
            self._as.supports_mrt()
        }

        /// Returns true if server supports the string-operations module
        /// (``STRING_READ`` op-type 17, ``STRING_MODIFY`` op-type 18,
        /// ``TO_STRING`` op-type 19), the ``CALL_STRING`` (module 3)
        /// string-expression dispatcher, and the dedicated ``TO_STRING``
        /// expression opcode. Requires server >= 8.2.0.
        pub fn supports_string_operations(&self) -> bool {
            self._as.supports_string_operations()
        }

        /// Returns true if server accepts server-compiled AEL on filter field 43 (>= 8.2.0.0).
        pub fn supports_server_compiled_ael(&self) -> bool {
            self._as.supports_server_compiled_ael()
        }

        /// Returns true if server supports two-phase server query selection
        /// (field ``44`` WHERE explain → execute). Requires server >= 8.2.0.
        pub fn supports_query_selection(&self) -> bool {
            self._as.supports_query_selection()
        }

        /// Returns true if server supports ``VECTOR`` values and vector-distance
        /// expressions. Requires server >= 8.2.1.
        pub fn supports_vector(&self) -> bool {
            self._as.supports_vector()
        }

        /// Returns true if server supports Top-K queries (the order-by and
        /// Top-K query fields). Requires server >= 8.2.1.
        pub fn supports_query_top_k(&self) -> bool {
            self._as.supports_query_top_k()
        }

        pub fn __str__(&self) -> String {
            format!("{}.{}.{}.{}", self._as.major, self._as.minor, self._as.patch, self._as.build)
        }

        pub fn __repr__(&self) -> String {
            format!("Version({}.{}.{}.{})", self._as.major, self._as.minor, self._as.patch, self._as.build)
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Node
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass)]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Clone)]
    pub struct Node {
        pub(crate) _as: std::sync::Arc<aerospike_core::Node>,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Node {
        /// Returns the node name.
        #[getter]
        pub fn name(&self) -> &str {
            self._as.name()
        }

        /// Returns the node address.
        #[getter]
        pub fn address(&self) -> &str {
            self._as.address()
        }

        /// Returns true if the node is active.
        #[getter]
        pub fn is_active(&self) -> bool {
            self._as.is_active()
        }

        /// Returns the server version.
        #[getter]
        pub fn version(&self) -> Version {
            Version { _as: self._as.version().clone() }
        }

        /// Returns the node host as a tuple (hostname, port).
        #[getter]
        pub fn host(&self) -> (String, u16) {
            let h = self._as.host();
            (h.name, h.port)
        }

        /// Returns the count of connection failures for this node.
        #[getter]
        pub fn failures(&self) -> usize {
            self._as.failures()
        }

        /// Returns the partition generation number.
        #[getter]
        pub fn partition_generation(&self) -> isize {
            self._as.partition_generation()
        }

        /// Returns the rebalance generation number.
        #[getter]
        pub fn rebalance_generation(&self) -> isize {
            self._as.rebalance_generation()
        }

        /// Returns a list of host aliases for this node.
        ///
        /// Reads the client's alias list (no network I/O), so it is a plain
        /// method callable from both async and sync code.
        pub fn aliases(&self) -> Vec<(String, u16)> {
            self._as.aliases().into_iter().map(|h| (h.name, h.port)).collect()
        }

        /// Execute an info command on this node.
        #[gen_stub(override_return_type(type_repr="typing.Awaitable[typing.Dict[str, str]]", imports=("typing")))]
        #[pyo3(signature = (command, *, policy = None))]
        pub fn info<'a>(
            &self,
            command: String,
            policy: Option<AdminPolicy>,
            py: Python<'a>,
        ) -> PyResult<Bound<'a, PyAny>> {
            let node = std::sync::Arc::clone(&self._as);
            let admin_policy =
                policy.map(|p| p._as).unwrap_or_default();

            pyo3_asyncio::future_into_py(py, async move {
                let response = node
                    .info(&admin_policy, &[&command])
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))?;
                Ok(response)
            })
        }

        /// Synchronously execute an info command on this node.
        #[pyo3(signature = (command, *, policy = None))]
        pub fn info_blocking(
            &self,
            command: String,
            policy: Option<AdminPolicy>,
            py: Python<'_>,
        ) -> PyResult<IndexMap<String, String>> {
            let node = std::sync::Arc::clone(&self._as);
            let admin_policy =
                policy.map(|p| p._as).unwrap_or_default();
            run_blocking(py, async move {
                node.info(&admin_policy, &[&command])
                    .await
                    .map_err(|e| PyErr::from(RustClientError(e)))
            })
        }

        pub fn __str__(&self) -> String {
            format!("Node(name={}, address={})", self._as.name(), self._as.address())
        }

        pub fn __repr__(&self) -> String {
            format!("Node(name='{}', address='{}', active={})", self._as.name(), self._as.address(), self._as.is_active())
        }
    }

    impl TryFrom<&PrivilegeCode> for aerospike_core::PrivilegeCode {
        type Error = PyErr;

        fn try_from(input: &PrivilegeCode) -> PyResult<Self> {
            Ok(match &input {
                PrivilegeCode::UserAdmin => aerospike_core::PrivilegeCode::UserAdmin,
                PrivilegeCode::SysAdmin => aerospike_core::PrivilegeCode::SysAdmin,
                PrivilegeCode::DataAdmin => aerospike_core::PrivilegeCode::DataAdmin,
                PrivilegeCode::UDFAdmin => aerospike_core::PrivilegeCode::UdfAdmin,
                PrivilegeCode::SIndexAdmin => aerospike_core::PrivilegeCode::SindexAdmin,
                PrivilegeCode::Read => aerospike_core::PrivilegeCode::Read,
                PrivilegeCode::ReadWrite => aerospike_core::PrivilegeCode::ReadWrite,
                PrivilegeCode::ReadWriteUDF => aerospike_core::PrivilegeCode::ReadWriteUdf,
                PrivilegeCode::Write => aerospike_core::PrivilegeCode::Write,
                PrivilegeCode::Truncate => aerospike_core::PrivilegeCode::Truncate,
                PrivilegeCode::MaskingAdmin => aerospike_core::PrivilegeCode::MaskingAdmin,
                PrivilegeCode::ReadMasked => aerospike_core::PrivilegeCode::ReadMasked,
                PrivilegeCode::WriteMasked => aerospike_core::PrivilegeCode::WriteMasked,
                PrivilegeCode::Unknown => {
                    return Err(crate::errors::ValueError::new_err(
                        "PrivilegeCode.Unknown cannot be granted: it only names a \
                         privilege reported by the server that this client has no name for",
                    ))
                }
            })
        }
    }

    impl From<&aerospike_core::PrivilegeCode> for PrivilegeCode {
        fn from(input: &aerospike_core::PrivilegeCode) -> Self {
            match &input {
                aerospike_core::PrivilegeCode::UserAdmin => PrivilegeCode::UserAdmin,
                aerospike_core::PrivilegeCode::SysAdmin => PrivilegeCode::SysAdmin,
                aerospike_core::PrivilegeCode::DataAdmin => PrivilegeCode::DataAdmin,
                aerospike_core::PrivilegeCode::UdfAdmin => PrivilegeCode::UDFAdmin,
                aerospike_core::PrivilegeCode::SindexAdmin => PrivilegeCode::SIndexAdmin,
                aerospike_core::PrivilegeCode::Read => PrivilegeCode::Read,
                aerospike_core::PrivilegeCode::ReadWrite => PrivilegeCode::ReadWrite,
                aerospike_core::PrivilegeCode::ReadWriteUdf => PrivilegeCode::ReadWriteUDF,
                aerospike_core::PrivilegeCode::Write => PrivilegeCode::Write,
                aerospike_core::PrivilegeCode::Truncate => PrivilegeCode::Truncate,
                aerospike_core::PrivilegeCode::MaskingAdmin => PrivilegeCode::MaskingAdmin,
                aerospike_core::PrivilegeCode::ReadMasked => PrivilegeCode::ReadMasked,
                aerospike_core::PrivilegeCode::WriteMasked => PrivilegeCode::WriteMasked,
                // A code the server reported without a name here, and a variant
                // core names that this build predates, read the same from Python.
                aerospike_core::PrivilegeCode::Unknown(_) | _ => PrivilegeCode::Unknown,
            }
        }
    }
    /**********************************************************************************
     *
     * User
     *
     **********************************************************************************/

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Clone)]
    pub struct User {
        pub(crate) _as: aerospike_core::User,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl User {
        #[getter]
        /// User name.
        pub fn get_user(&self) -> String {
            self._as.user.clone()
        }

        #[getter]
        /// List of assigned roles.
        pub fn get_roles(&self) -> Vec<String> {
            self._as.roles.clone()
        }

        #[getter]
        /// List of read statistics. List may be nil.
        /// Current statistics by offset are:
        ///
        /// 0: read quota in records per second
        /// 1: single record read command rate (TPS)
        /// 2: read scan/query record per second rate (RPS)
        /// 3: number of limitless read scans/queries
        ///
        /// Future server releases may add additional statistics.
        pub fn get_read_info(&self) -> Vec<u32> {
            self._as.read_info.clone()
        }

        #[getter]
        /// List of write statistics. List may be nil.
        /// Current statistics by offset are:
        ///
        /// 0: write quota in records per second
        /// 1: single record write command rate (TPS)
        /// 2: write scan/query record per second rate (RPS)
        /// 3: number of limitless write scans/queries
        ///
        /// Future server releases may add additional statistics.
        pub fn get_write_info(&self) -> Vec<u32> {
            self._as.write_info.clone()
        }

        #[getter]
        /// Number of currently open connections for the user
        pub fn get_conns_in_user(&self) -> u32 {
            self._as.conns_in_use
        }
    }

    /**********************************************************************************
     *
     * Role
     *
     **********************************************************************************/

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, subclass, module = "_native")]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Clone)]
    pub struct Role {
        pub(crate) _as: aerospike_core::Role,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Role {
        #[getter]
        /// Role name.
        pub fn get_name(&self) -> String {
            self._as.name.clone()
        }

        #[getter]
        /// List of assigned privileges.
        pub fn get_privileges(&self) -> Vec<Privilege> {
            self._as
                .privileges
                .iter()
                .map(|p| Privilege { _as: p.clone() })
                .collect()
        }

        #[getter]
        /// The list of allowable IP addresses.
        pub fn get_allowlist(&self) -> Vec<String> {
            self._as.allowlist.clone()
        }

        #[getter]
        /// Maximum reads per second limit for the role.
        pub fn get_read_quota(&self) -> u32 {
            self._as.read_quota
        }

        #[getter]
        /// Maximum writes per second limit for the role.
        pub fn get_write_quota(&self) -> u32 {
            self._as.write_quota
        }
    }

    /**********************************************************************************
     *
     * Privilege
     *
     **********************************************************************************/

    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object,
        name = "Privilege",
        module = "_native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 1))]
    #[derive(Clone)]
    pub struct Privilege {
        pub(crate) _as: aerospike_core::Privilege,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Privilege {
        #[new]
        pub fn __construct(
            code: &PrivilegeCode,
            namespace: Option<String>,
            set_name: Option<String>,
        ) -> PyResult<Self> {
            Ok(Privilege {
                _as: aerospike_core::Privilege::new(code.try_into()?, namespace, set_name),
            })
        }

        #[getter]
        pub fn get_code(&self) -> PrivilegeCode {
            (&self._as.code).into()
        }

        #[getter]
        pub fn get_namespace(&self) -> Option<String> {
            self._as.namespace.clone()
        }

        #[getter]
        pub fn get_set_name(&self) -> Option<String> {
            self._as.set_name.clone()
        }

        fn as_string(&self) -> String {
            match (&self._as.namespace, &self._as.set_name) {
                (Some(ns), Some(set)) => format!("{}:{}.{}", self._as.code, ns, set),
                (Some(ns), None) => format!("{}:{}", self._as.code, ns),
                (None, _) => format!("{}", self._as.code),
            }
        }

        fn __str__(&self) -> PyResult<String> {
            Ok(self.as_string())
        }

        fn __repr__(&self) -> PyResult<String> {
            let s = self.__str__()?;
            Ok(format!("Privilege({})", s))
        }
    }


    impl fmt::Display for Privilege {
        fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
            write!(f, "{}", self.as_string())
        }
    }
