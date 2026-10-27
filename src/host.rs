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

use aerospike_core::ToHosts;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use pyo3_stub_gen::{PyStubType, TypeInfo};

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Host
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// Seed address of a cluster node.
    ///
    /// Pass a list of these to :func:`new_client` (or :func:`new_client_blocking`)
    /// instead of a seed string. Each field is taken as-is, so an IPv6 address
    /// or a TLS name needs no quoting or bracketing.
    ///
    /// Example::
    ///
    ///     seeds = [Host("10.0.0.1", 3000), Host("fd00::2", 3000)]
    ///     client = await new_client(ClientPolicy(), seeds)
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, module = "_native", frozen, eq, hash)]
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub struct Host {
        pub(crate) _as: aerospike_core::Host,
    }

    #[gen_stub_pymethods]
    #[pymethods]
    impl Host {
        /// Create a Host.
        ///
        /// Args:
        ///     name: Hostname or IP address. An IPv6 address is given bare,
        ///         e.g. ``"fd00::2"``, not ``"[fd00::2]"``.
        ///     port: Service port.
        ///     tls_name: Name the node's TLS certificate must carry. ``None``
        ///         (or blank) for a host reached without TLS.
        ///
        /// Returns:
        ///     Host
        ///
        /// Raises:
        ///     ValueError: If *name* is empty or bracketed.
        #[new]
        #[pyo3(signature = (name, port, tls_name=None))]
        pub fn new(name: &str, port: u16, tls_name: Option<&str>) -> PyResult<Self> {
            if name.is_empty() {
                return Err(PyValueError::new_err("Host name must not be empty"));
            }
            // A bracketed name would be resolved literally, brackets included,
            // and fail as an unknown hostname far from the cause.
            if name.starts_with('[') {
                return Err(PyValueError::new_err(format!(
                    "Host name {name:?} is bracketed; give the IPv6 address bare"
                )));
            }
            let host = match tls_name {
                Some(tls_name) => aerospike_core::Host::new_tls(name, tls_name, port),
                None => aerospike_core::Host::new(name, port),
            };
            Ok(Host { _as: host })
        }

        /// Hostname or IP address.
        #[getter]
        pub fn name(&self) -> &str {
            &self._as.name
        }

        /// Service port.
        #[getter]
        pub fn port(&self) -> u16 {
            self._as.port
        }

        /// TLS certificate name, or ``None`` for a host reached without TLS.
        #[getter]
        pub fn tls_name(&self) -> Option<&str> {
            self._as.tls_name.as_deref()
        }

        /// Seed-string form: ``name[:tls_name]:port``, IPv6 bracketed.
        pub fn __str__(&self) -> String {
            seed_entry(&self._as)
        }

        pub fn __repr__(&self) -> String {
            match &self._as.tls_name {
                Some(tls_name) => format!(
                    "Host('{}', {}, tls_name='{}')",
                    self._as.name, self._as.port, tls_name
                ),
                None => format!("Host('{}', {})", self._as.name, self._as.port),
            }
        }
    }

    /// Format *host* as one seed-string entry, bracketing an IPv6 name so its
    /// colons are not read as field separators.
    fn seed_entry(host: &aerospike_core::Host) -> String {
        let (open, close) = if host.name.contains(':') { ("[", "]") } else { ("", "") };
        match &host.tls_name {
            Some(tls_name) => format!("{open}{}{close}:{tls_name}:{}", host.name, host.port),
            None => format!("{open}{}{close}:{}", host.name, host.port),
        }
    }

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  Seeds
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    /// The seed argument of the client constructors: a seed string, parsed by
    /// the core, or a non-empty sequence of :class:`Host`, passed through.
    pub enum Seeds {
        Text(String),
        Hosts(Vec<aerospike_core::Host>),
    }

    impl Seeds {
        pub(crate) fn as_to_hosts(&self) -> &(dyn ToHosts + Send + Sync) {
            match self {
                Seeds::Text(text) => text,
                Seeds::Hosts(hosts) => hosts,
            }
        }

        /// Seed-string form, as reported by :meth:`Client.seeds`.
        pub(crate) fn to_seed_string(&self) -> String {
            match self {
                Seeds::Text(text) => text.clone(),
                Seeds::Hosts(hosts) => {
                    hosts.iter().map(seed_entry).collect::<Vec<_>>().join(",")
                }
            }
        }
    }

    impl<'a, 'py> FromPyObject<'a, 'py> for Seeds {
        type Error = PyErr;

        fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
            if let Ok(text) = obj.extract::<String>() {
                return Ok(Seeds::Text(text));
            }
            let hosts: Vec<Host> = obj.extract().map_err(|_| {
                PyTypeError::new_err("seeds must be a str or a sequence of Host")
            })?;
            // With no seeds the core raises a connection error that does not
            // say why, or, with `fail_if_not_connected` off, returns a client
            // that can never reach a node.
            if hosts.is_empty() {
                return Err(PyValueError::new_err("seeds must contain at least one Host"));
            }
            Ok(Seeds::Hosts(hosts.into_iter().map(|host| host._as).collect()))
        }
    }

    impl PyStubType for Seeds {
        fn type_output() -> TypeInfo {
            String::type_input() | Vec::<Host>::type_input()
        }
    }
