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

use std::sync::Arc;

use aerospike_core::discovery::dns::{
    self as core_dns, DnsSeedCandidateProvider as CoreDnsProvider,
};
use aerospike_core::discovery::{
    PassThroughAddressTranslator as CorePassThrough, PortMapAddressTranslator as CorePortMap,
    SharedAddressTranslator, SharedSeedCandidateProvider,
    StaticMapAddressTranslator as CoreStaticMap, StaticSeedCandidateProvider as CoreStaticProvider,
};
use aerospike_core::ErrorKind;
use indexmap::IndexMap;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use pyo3_stub_gen::{PyStubType, TypeInfo};

use crate::errors::RustClientError;
use crate::{DnsRecordType, Host};

/// A rejected setting is Python's `ValueError`, as for any constructor: it
/// never reached a server, so there is no result code to carry.
fn setting_error(e: aerospike_core::Error) -> PyErr {
    match e.kind() {
        ErrorKind::InvalidArgument => {
            PyValueError::new_err(e.message().unwrap_or_default().to_owned())
        }
        _ => RustClientError(e).into(),
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  Seed candidate providers
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Seed candidate provider over a fixed list of hosts.
///
/// Called once, at client construction. Combined with the seeds passed
/// to the client constructor by :attr:`ClientPolicy.discovery_merge_policy`.
///
/// Example::
///
///     policy = ClientPolicy()
///     policy.seed_candidate_provider = StaticSeedCandidateProvider(
///         [Host("10.0.0.1", 3000), Host("10.0.0.2", 3000)]
///     )
///     client = await new_client(policy, [])
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, module = "_native", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct StaticSeedCandidateProvider {
    _as: Arc<CoreStaticProvider>,
}

#[gen_stub_pymethods]
#[pymethods]
impl StaticSeedCandidateProvider {
    /// Create a provider over *hosts*.
    ///
    /// Args:
    ///     hosts: The seed hosts the provider returns.
    ///
    /// Returns:
    ///     StaticSeedCandidateProvider
    #[new]
    pub fn new(hosts: Vec<Host>) -> Self {
        let hosts: Vec<aerospike_core::Host> = hosts.into_iter().map(|host| host._as).collect();
        StaticSeedCandidateProvider {
            _as: Arc::new(CoreStaticProvider::new(hosts)),
        }
    }

    /// The seed hosts the provider returns.
    #[getter]
    pub fn hosts(&self) -> Vec<Host> {
        self._as
            .hosts()
            .iter()
            .map(|host| Host { _as: host.clone() })
            .collect()
    }

    pub fn __repr__(&self) -> String {
        let hosts: Vec<String> = self.hosts().iter().map(Host::__repr__).collect();
        format!("StaticSeedCandidateProvider([{}])", hosts.join(", "))
    }
}

/// Seed candidate provider that resolves DNS names into seed hosts.
///
/// Resolved at client construction and again every
/// :attr:`ClientPolicy.discovery_refresh_interval` milliseconds, on the
/// cluster tend task. Every returned host is an IP address. A refresh
/// must finish within :attr:`ClientPolicy.discovery_tend_refresh_deadline`,
/// so *timeout* may not exceed it: client construction fails otherwise.
/// With the default deadline of 100 ms, only a nearby resolver answers
/// reliably in time.
///
/// A name that does not resolve is logged at warning level and the
/// others are used. When every name fails, a refresh keeps the previous
/// seeds, and at construction the static seeds are used; with no static
/// seeds either, construction fails with
/// ``ResultCode.SEED_DISCOVERY_FAILED``.
///
/// Example::
///
///     policy = ClientPolicy()
///     policy.seed_candidate_provider = DnsSeedCandidateProvider(
///         ["_aerospike._tcp.prod.example.com"],
///         record_type=DnsRecordType.SRV,
///         timeout=50,
///         servers=["10.20.0.2"],
///     )
///     policy.discovery_merge_policy = SeedMergePolicy.DISCOVERY_ONLY
///     client = await new_client(policy, [])
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, module = "_native", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct DnsSeedCandidateProvider {
    _as: Arc<CoreDnsProvider>,
}

#[gen_stub_pymethods]
#[pymethods]
impl DnsSeedCandidateProvider {
    /// Create a provider resolving *names*.
    ///
    /// Args:
    ///     names: DNS names to resolve, at least one, without duplicates
    ///         (compared case-insensitively, trailing dot optional). Each
    ///         is queried as an absolute name.
    ///     record_type: Record type queried for each name.
    ///     port: Port given to hosts resolved from A and AAAA records;
    ///         SRV records carry their own.
    ///     tls_name: TLS name given to every returned host. ``None`` to
    ///         default it as for the constructor's seeds.
    ///     timeout: Total time in milliseconds for one refresh, across
    ///         every name, record and resolver.
    ///     servers: Resolver addresses, each an IP address with an
    ///         optional port (``"10.0.0.2"``, ``"10.0.0.2:5353"``,
    ///         ``"[2001:db8::1]:53"``), tried in order. ``None`` for the
    ///         system resolvers, read once here.
    ///
    /// Returns:
    ///     DnsSeedCandidateProvider
    ///
    /// Raises:
    ///     ValueError: If a setting is invalid; the message names it.
    ///     ClientError: If *servers* is ``None`` and no system resolver
    ///         is configured.
    #[new]
    #[pyo3(signature = (
            names,
            record_type = DnsRecordType::A,
            port = core_dns::DEFAULT_PORT,
            tls_name = None,
            timeout = core_dns::DEFAULT_TIMEOUT_MS,
            servers = None,
        ))]
    pub fn new(
        names: Vec<String>,
        record_type: DnsRecordType,
        port: u16,
        tls_name: Option<String>,
        timeout: u32,
        servers: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let mut builder = CoreDnsProvider::builder()
            .names(names)
            .record_type(record_type.into())
            .port(port)
            .timeout(timeout)
            .servers(servers.unwrap_or_default());
        if let Some(tls_name) = tls_name {
            builder = builder.tls_name(tls_name);
        }
        let provider = builder.build().map_err(setting_error)?;
        Ok(DnsSeedCandidateProvider {
            _as: Arc::new(provider),
        })
    }

    /// The DNS names resolved on every refresh.
    #[getter]
    pub fn names(&self) -> Vec<String> {
        self._as.names().to_vec()
    }

    /// The record type queried.
    #[getter]
    pub fn record_type(&self) -> DnsRecordType {
        self._as.record_type().into()
    }

    /// The port given to hosts resolved from A and AAAA records.
    #[getter]
    pub fn port(&self) -> u16 {
        self._as.port()
    }

    /// The TLS name given to every returned host, or ``None``.
    #[getter]
    pub fn tls_name(&self) -> Option<&str> {
        self._as.tls_name()
    }

    /// Total time in milliseconds for one refresh.
    #[getter]
    pub fn timeout(&self) -> u32 {
        self._as.timeout()
    }

    /// The configured resolver addresses; empty for the system resolvers.
    #[getter]
    pub fn servers(&self) -> Vec<String> {
        self._as.servers().to_vec()
    }

    pub fn __repr__(&self) -> String {
        let quoted = |values: &[String]| {
            values
                .iter()
                .map(|v| format!("'{v}'"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let tls_name = match self._as.tls_name() {
            Some(tls_name) => format!("'{tls_name}'"),
            None => "None".to_string(),
        };
        format!(
            "DnsSeedCandidateProvider([{}], record_type=DnsRecordType.{}, port={}, \
                 tls_name={tls_name}, timeout={}, servers=[{}])",
            quoted(self._as.names()),
            self._as.record_type().as_str().to_ascii_uppercase(),
            self._as.port(),
            self._as.timeout(),
            quoted(self._as.servers()),
        )
    }
}

/// A value of :attr:`ClientPolicy.seed_candidate_provider`.
#[derive(Clone)]
pub enum SeedProvider {
    Static(StaticSeedCandidateProvider),
    Dns(DnsSeedCandidateProvider),
}

impl SeedProvider {
    pub(crate) fn to_core(&self) -> SharedSeedCandidateProvider {
        match self {
            SeedProvider::Static(p) => p._as.clone(),
            SeedProvider::Dns(p) => p._as.clone(),
        }
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for SeedProvider {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(p) = obj.extract::<StaticSeedCandidateProvider>() {
            return Ok(SeedProvider::Static(p));
        }
        if let Ok(p) = obj.extract::<DnsSeedCandidateProvider>() {
            return Ok(SeedProvider::Dns(p));
        }
        Err(PyTypeError::new_err(
            "seed_candidate_provider must be a StaticSeedCandidateProvider or a \
                 DnsSeedCandidateProvider",
        ))
    }
}

impl<'py> IntoPyObject<'py> for SeedProvider {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self {
            SeedProvider::Static(p) => Ok(Bound::new(py, p)?.into_any()),
            SeedProvider::Dns(p) => Ok(Bound::new(py, p)?.into_any()),
        }
    }
}

impl PyStubType for SeedProvider {
    fn type_output() -> TypeInfo {
        StaticSeedCandidateProvider::type_output() | DnsSeedCandidateProvider::type_output()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////
//
//  Address translators
//
////////////////////////////////////////////////////////////////////////////////////////////

/// Address translator mapping an advertised host name or IP address to
/// another; the port is kept.
///
/// Set as :attr:`ClientPolicy.address_translator`, it replaces
/// :attr:`ClientPolicy.ip_map`, which builds the same translator.
///
/// Example::
///
///     policy = ClientPolicy()
///     policy.address_translator = StaticMapAddressTranslator(
///         {"10.0.0.1": "203.0.113.1", "10.0.0.2": "203.0.113.2"}
///     )
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, module = "_native", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct StaticMapAddressTranslator {
    ip_map: IndexMap<String, String>,
    _as: Arc<CoreStaticMap>,
}

#[gen_stub_pymethods]
#[pymethods]
impl StaticMapAddressTranslator {
    /// Create a translator over *ip_map*.
    ///
    /// Args:
    ///     ip_map: Advertised host to the host to connect to. An
    ///         advertised host not in the map is a miss, handled by
    ///         :attr:`ClientPolicy.translation_on_miss`.
    ///
    /// Returns:
    ///     StaticMapAddressTranslator
    #[new]
    pub fn new(ip_map: IndexMap<String, String>) -> Self {
        let core_map = ip_map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        StaticMapAddressTranslator {
            ip_map,
            _as: Arc::new(CoreStaticMap::new(Some(core_map))),
        }
    }

    /// The advertised host to connect-to host map.
    #[getter]
    pub fn ip_map(&self) -> IndexMap<String, String> {
        self.ip_map.clone()
    }

    pub fn __repr__(&self) -> String {
        format!("StaticMapAddressTranslator({})", map_repr(&self.ip_map))
    }
}

/// Address translator mapping an advertised ``host:port``, or a node
/// name, to the ``host:port`` to connect to.
///
/// For a load balancer or NAT that gives each node its own port on a
/// shared address. A ``host:port`` key takes precedence over a node name
/// key; an IPv6 host is written in brackets (``"[2001:db8::1]:3000"``).
///
/// Example::
///
///     policy = ClientPolicy()
///     policy.address_translator = PortMapAddressTranslator({
///         "10.0.0.1:3000": "lb.example.com:30001",
///         "BB9020011AC4202": "lb.example.com:30002",
///     })
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, module = "_native", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct PortMapAddressTranslator {
    port_map: IndexMap<String, String>,
    _as: Arc<CorePortMap>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PortMapAddressTranslator {
    /// Create a translator over *port_map*.
    ///
    /// Args:
    ///     port_map: Advertised ``host:port`` or node name to the
    ///         ``host:port`` to connect to, the port in 1 to 65535. An
    ///         advertised address matching no key is a miss, handled by
    ///         :attr:`ClientPolicy.translation_on_miss`.
    ///
    /// Returns:
    ///     PortMapAddressTranslator
    ///
    /// Raises:
    ///     ValueError: If an entry is invalid; the message names it.
    #[new]
    pub fn new(port_map: IndexMap<String, String>) -> PyResult<Self> {
        let translator = CorePortMap::new(&port_map).map_err(setting_error)?;
        Ok(PortMapAddressTranslator {
            port_map,
            _as: Arc::new(translator),
        })
    }

    /// The advertised address to connect-to address map.
    #[getter]
    pub fn port_map(&self) -> IndexMap<String, String> {
        self.port_map.clone()
    }

    pub fn __repr__(&self) -> String {
        format!("PortMapAddressTranslator({})", map_repr(&self.port_map))
    }
}

/// Address translator returning every advertised address unchanged.
///
/// For servers that advertise a client-reachable address and port per
/// node, such as with :attr:`ClientPolicy.use_services_alternate`. Never
/// misses.
///
/// Example::
///
///     policy = ClientPolicy()
///     policy.use_services_alternate = True
///     policy.address_translator = PassThroughAddressTranslator()
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, module = "_native", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct PassThroughAddressTranslator {
    _as: Arc<CorePassThrough>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PassThroughAddressTranslator {
    /// Create a pass-through translator.
    ///
    /// Returns:
    ///     PassThroughAddressTranslator
    #[new]
    pub fn new() -> Self {
        PassThroughAddressTranslator {
            _as: Arc::new(CorePassThrough),
        }
    }

    pub fn __repr__(&self) -> &'static str {
        "PassThroughAddressTranslator()"
    }
}

fn map_repr(map: &IndexMap<String, String>) -> String {
    let entries: Vec<String> = map.iter().map(|(k, v)| format!("'{k}': '{v}'")).collect();
    format!("{{{}}}", entries.join(", "))
}

/// A value of :attr:`ClientPolicy.address_translator`.
#[derive(Clone)]
pub enum Translator {
    StaticMap(StaticMapAddressTranslator),
    PortMap(PortMapAddressTranslator),
    PassThrough(PassThroughAddressTranslator),
}

impl Translator {
    pub(crate) fn to_core(&self) -> SharedAddressTranslator {
        match self {
            Translator::StaticMap(t) => t._as.clone(),
            Translator::PortMap(t) => t._as.clone(),
            Translator::PassThrough(t) => t._as.clone(),
        }
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for Translator {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if let Ok(t) = obj.extract::<StaticMapAddressTranslator>() {
            return Ok(Translator::StaticMap(t));
        }
        if let Ok(t) = obj.extract::<PortMapAddressTranslator>() {
            return Ok(Translator::PortMap(t));
        }
        if let Ok(t) = obj.extract::<PassThroughAddressTranslator>() {
            return Ok(Translator::PassThrough(t));
        }
        Err(PyTypeError::new_err(
            "address_translator must be a StaticMapAddressTranslator, a \
                 PortMapAddressTranslator or a PassThroughAddressTranslator",
        ))
    }
}

impl<'py> IntoPyObject<'py> for Translator {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self {
            Translator::StaticMap(t) => Ok(Bound::new(py, t)?.into_any()),
            Translator::PortMap(t) => Ok(Bound::new(py, t)?.into_any()),
            Translator::PassThrough(t) => Ok(Bound::new(py, t)?.into_any()),
        }
    }
}

impl PyStubType for Translator {
    fn type_output() -> TypeInfo {
        StaticMapAddressTranslator::type_output()
            | PortMapAddressTranslator::type_output()
            | PassThroughAddressTranslator::type_output()
    }
}
