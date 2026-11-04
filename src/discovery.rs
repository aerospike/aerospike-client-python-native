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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use aerospike_core::discovery::dns::{
    self as core_dns, DnsSeedCandidateProvider as CoreDnsProvider,
};
use aerospike_core::discovery::{
    PassThroughAddressTranslator as CorePassThrough, PortMapAddressTranslator as CorePortMap,
    SeedCandidateContext as CoreContext, SeedCandidateProvider, SharedAddressTranslator,
    SharedSeedCandidateProvider, StaticMapAddressTranslator as CoreStaticMap,
    StaticSeedCandidateProvider as CoreStaticProvider,
};
use aerospike_core::{Error as CoreError, ErrorKind, Host as CoreHost, Result as CoreResult};
use indexmap::IndexMap;
use log::warn;
use parking_lot::Mutex;
use pyo3::exceptions::{PyNotImplementedError, PyTypeError, PyValueError};
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use pyo3_stub_gen::{PyStubType, TypeInfo};

#[cfg(feature = "discovery-aws")]
use aerospike_core::discovery::aws::{
    self as core_aws, AwsEc2TagSeedCandidateProvider as CoreAwsProvider,
    Credentials as CoreCredentials, CredentialsHook as CoreCredentialsHook,
};
#[cfg(feature = "discovery-aws")]
use pyo3::types::{PyDateTime, PyTzInfo};

use crate::errors::RustClientError;
#[cfg(feature = "discovery-aws")]
use crate::AwsAddressField;
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

/// AWS credentials an :class:`AwsEc2TagSeedCandidateProvider` signs its
/// requests with. Its ``repr`` shows only the access key id.
///
/// Example::
///
///     frozen = boto3.Session().get_credentials().get_frozen_credentials()
///     credentials = AwsCredentials(frozen.access_key, frozen.secret_key, frozen.token)
#[cfg(feature = "discovery-aws")]
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, module = "_native", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct AwsCredentials {
    _as: CoreCredentials,
}

#[cfg(feature = "discovery-aws")]
#[gen_stub_pymethods]
#[pymethods]
impl AwsCredentials {
    /// Create credentials.
    ///
    /// Args:
    ///     access_key_id: The access key id.
    ///     secret_access_key: The secret access key.
    ///     session_token: The session token of temporary credentials.
    ///     expiration: When temporary credentials expire, as a
    ///         timezone-aware datetime. They are used until five minutes
    ///         before it. ``None`` for credentials that do not expire.
    ///
    /// Returns:
    ///     AwsCredentials
    ///
    /// Raises:
    ///     ValueError: If the access key id or the secret is empty.
    #[new]
    #[pyo3(signature = (access_key_id, secret_access_key, session_token = None, expiration = None))]
    pub fn new(
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        expiration: Option<&Bound<'_, PyDateTime>>,
    ) -> PyResult<Self> {
        let expiration_millis = match expiration {
            Some(expiration) => {
                let seconds: f64 = expiration
                    .call_method0(intern!(expiration.py(), "timestamp"))?
                    .extract()?;
                (seconds * 1000.0) as i64
            }
            None => 0,
        };
        let credentials = CoreCredentials::with_expiration(
            access_key_id,
            secret_access_key,
            session_token,
            expiration_millis,
        )
        .map_err(setting_error)?;
        Ok(AwsCredentials { _as: credentials })
    }

    /// The access key id.
    #[getter]
    pub fn access_key_id(&self) -> &str {
        self._as.access_key_id()
    }

    /// When the credentials expire, in UTC, or ``None``.
    #[getter]
    pub fn expiration<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDateTime>>> {
        match self._as.expiration() {
            0 => Ok(None),
            millis => {
                let utc = PyTzInfo::utc(py)?;
                PyDateTime::from_timestamp(py, millis as f64 / 1000.0, Some(&utc)).map(Some)
            }
        }
    }

    pub fn __repr__(&self) -> String {
        format!("AwsCredentials(access_key_id='{}', ...)", self._as.access_key_id())
    }
}

/// Seed candidate provider that returns one host per running EC2 instance
/// matching a set of filters, usually a tag.
///
/// For a self-managed cluster on EC2 whose nodes carry a known tag. It sees
/// only the instances of the credentials' own account and region, so it
/// does not discover Aerospike Cloud nodes. Each refresh sends one signed
/// ``DescribeInstances`` request with *filters* plus
/// ``instance-state-name=running``, and returns one host per instance that
/// has *address_field*, sorted by address. The credentials need only the
/// ``ec2:DescribeInstances`` permission.
///
/// Called at client construction, within *init_timeout*, and again every
/// :attr:`ClientPolicy.discovery_refresh_interval` milliseconds on the
/// cluster tend task, within *timeout*. *timeout* may not exceed
/// :attr:`ClientPolicy.discovery_tend_refresh_deadline`: client
/// construction fails otherwise. A failed refresh keeps the previous seeds.
///
/// Credentials are, in order, the first found of: *credentials*; the
/// ``AWS_ACCESS_KEY_ID``, ``AWS_SECRET_ACCESS_KEY`` and
/// ``AWS_SESSION_TOKEN`` environment variables; the static keys of profile
/// ``AWS_PROFILE`` in the shared credentials file; EKS web identity; ECS
/// container credentials; the EC2 instance profile. For any other source,
/// such as SSO or an assumed role, pass *credentials_hook*.
///
/// Example::
///
///     policy = ClientPolicy()
///     policy.seed_candidate_provider = AwsEc2TagSeedCandidateProvider(
///         "us-east-1", {"tag:aerospike-cluster": ["prod-a"]}
///     )
///     client = await new_client(policy, [Host("aerospike.example.com", 3000)])
#[cfg(feature = "discovery-aws")]
#[gen_stub_pyclass(module = "_native")]
#[pyclass(from_py_object, module = "_native", frozen, eq)]
#[derive(Clone)]
pub struct AwsEc2TagSeedCandidateProvider {
    _as: Arc<CoreAwsProvider>,
    credentials: Option<AwsCredentials>,
    credentials_hook: Option<Arc<Py<PyAny>>>,
}

/// Equal only to itself: the provider holds its credential cache.
#[cfg(feature = "discovery-aws")]
impl PartialEq for AwsEc2TagSeedCandidateProvider {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self._as, &other._as)
    }
}

#[cfg(feature = "discovery-aws")]
#[gen_stub_pymethods]
#[pymethods]
impl AwsEc2TagSeedCandidateProvider {
    /// Create a provider listing the instances in *region* that match
    /// *filters*.
    ///
    /// Args:
    ///     region: AWS region of the instances, such as ``"us-east-1"``.
    ///     filters: ``DescribeInstances`` filters, each a filter name and
    ///         its values, at least one. ``tag:<key>`` matches a tag; any
    ///         other EC2 filter name is passed as given. An instance
    ///         matches when it matches one value of every filter.
    ///     address_field: Instance field returned as each seed address.
    ///     port: Port given to every host.
    ///     tls_name: TLS name given to every host. ``None`` to default it
    ///         as for the constructor's seeds.
    ///     timeout: Total time in milliseconds for one periodic refresh,
    ///         credentials included.
    ///     init_timeout: Total time in milliseconds for the refresh at
    ///         client construction, which also finds the credentials.
    ///     endpoint: EC2 endpoint URL, such as a VPC interface endpoint.
    ///         ``None`` for the regional endpoint.
    ///     credentials: Credentials to use before the rest of the chain.
    ///     credentials_hook: A callable taking no arguments and returning
    ///         :class:`AwsCredentials`, replacing the chain. It is called
    ///         at the first refresh and again when the credentials it
    ///         returned are within five minutes of their expiration, on
    ///         the task running the refresh, so its time counts against
    ///         *timeout*: return credentials refreshed elsewhere, with
    ///         their expiration, rather than call AWS from the hook.
    ///
    /// Returns:
    ///     AwsEc2TagSeedCandidateProvider
    ///
    /// Raises:
    ///     ValueError: If a setting is invalid, or both *credentials* and
    ///         *credentials_hook* are set; the message names it.
    ///     TypeError: If *credentials_hook* is not callable.
    #[new]
    #[pyo3(signature = (
            region,
            filters,
            address_field = AwsAddressField::PrivateIp,
            port = core_aws::DEFAULT_PORT,
            tls_name = None,
            timeout = core_aws::DEFAULT_TIMEOUT_MS,
            init_timeout = core_aws::DEFAULT_INIT_TIMEOUT_MS,
            endpoint = None,
            credentials = None,
            credentials_hook = None,
        ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        region: String,
        filters: IndexMap<String, Vec<String>>,
        address_field: AwsAddressField,
        port: u16,
        tls_name: Option<String>,
        timeout: u32,
        init_timeout: u32,
        endpoint: Option<String>,
        credentials: Option<AwsCredentials>,
        credentials_hook: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let mut builder = CoreAwsProvider::builder()
            .region(region)
            .address_field(address_field.into())
            .port(port)
            .timeout(timeout)
            .init_timeout(init_timeout);
        for (name, values) in filters {
            builder = builder.filter(name, values);
        }
        if let Some(tls_name) = tls_name {
            builder = builder.tls_name(tls_name);
        }
        if let Some(endpoint) = endpoint {
            builder = builder.endpoint(endpoint);
        }
        if let Some(credentials) = &credentials {
            builder = builder.credentials(credentials._as.clone());
        }
        let credentials_hook = match credentials_hook {
            Some(hook) if !hook.is_callable() => {
                return Err(PyTypeError::new_err(format!(
                    "credentials_hook must be callable, not {}",
                    hook.get_type().qualname()?
                )))
            }
            Some(hook) => {
                let hook = Arc::new(hook.unbind());
                builder = builder.credentials_hook(python_credentials_hook(hook.clone()));
                Some(hook)
            }
            None => None,
        };
        let provider = builder.build().map_err(setting_error)?;
        Ok(AwsEc2TagSeedCandidateProvider {
            _as: Arc::new(provider),
            credentials,
            credentials_hook,
        })
    }

    /// The AWS region.
    #[getter]
    pub fn region(&self) -> &str {
        self._as.region()
    }

    /// The ``DescribeInstances`` filters, in configured order.
    #[getter]
    pub fn filters(&self) -> IndexMap<String, Vec<String>> {
        self._as.filters().iter().cloned().collect()
    }

    /// The instance field returned as each seed address.
    #[getter]
    pub fn address_field(&self) -> AwsAddressField {
        self._as.address_field().into()
    }

    /// The port given to every host.
    #[getter]
    pub fn port(&self) -> u16 {
        self._as.port()
    }

    /// The TLS name given to every host, or ``None``.
    #[getter]
    pub fn tls_name(&self) -> Option<&str> {
        self._as.tls_name()
    }

    /// Total time in milliseconds for one periodic refresh.
    #[getter]
    pub fn timeout(&self) -> u32 {
        self._as.timeout()
    }

    /// Total time in milliseconds for the refresh at client construction.
    #[getter]
    pub fn init_timeout(&self) -> u32 {
        self._as.init_timeout()
    }

    /// The configured EC2 endpoint, or ``None`` for the regional endpoint.
    #[getter]
    pub fn endpoint(&self) -> Option<&str> {
        self._as.endpoint()
    }

    /// The credentials passed to the constructor, or ``None``.
    #[getter]
    pub fn credentials(&self) -> Option<AwsCredentials> {
        self.credentials.clone()
    }

    /// The credentials hook passed to the constructor, or ``None``.
    #[getter]
    pub fn credentials_hook(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.credentials_hook.as_ref().map(|hook| hook.clone_ref(py))
    }

    pub fn __repr__(&self) -> String {
        let filters: Vec<String> = self
            ._as
            .filters()
            .iter()
            .map(|(name, values)| {
                let values: Vec<String> = values.iter().map(|v| format!("'{v}'")).collect();
                format!("'{name}': [{}]", values.join(", "))
            })
            .collect();
        let quoted = |value: Option<&str>| match value {
            Some(value) => format!("'{value}'"),
            None => "None".to_string(),
        };
        format!(
            "AwsEc2TagSeedCandidateProvider('{}', {{{}}}, address_field=AwsAddressField.{}, \
             port={}, tls_name={}, timeout={}, init_timeout={}, endpoint={})",
            self._as.region(),
            filters.join(", "),
            self._as.address_field().as_str().to_ascii_uppercase(),
            self._as.port(),
            quoted(self._as.tls_name()),
            self._as.timeout(),
            self._as.init_timeout(),
            quoted(self._as.endpoint()),
        )
    }
}

/// Calls a Python credentials hook from the refreshing task.
#[cfg(feature = "discovery-aws")]
fn python_credentials_hook(hook: Arc<Py<PyAny>>) -> CoreCredentialsHook {
    Arc::new(move || {
        Python::try_attach(|py| {
            let result = hook
                .bind(py)
                .call0()
                .map_err(|e| CoreError::client_error(e.to_string()))?;
            let credentials = result.extract::<AwsCredentials>().map_err(|_| {
                let type_name = result.get_type().qualname().map(|n| n.to_string());
                CoreError::client_error(format!(
                    "it returned {}, not AwsCredentials",
                    type_name.unwrap_or_default()
                ))
            })?;
            Ok(credentials._as)
        })
        .unwrap_or_else(|| Err(CoreError::client_error("the Python interpreter is shutting down")))
    })
}

/// What a :class:`SeedCandidateProvider` is told about the call it answers.
#[gen_stub_pyclass(module = "_native")]
#[pyclass(module = "_native", frozen)]
pub struct SeedCandidateContext {
    _as: CoreContext,
}

#[gen_stub_pymethods]
#[pymethods]
impl SeedCandidateContext {
    /// The expected cluster name, :attr:`ClientPolicy.cluster_name`.
    #[getter]
    pub fn cluster_name(&self) -> Option<&str> {
        self._as.cluster_name.as_deref()
    }

    /// The seeds passed to the client constructor.
    #[getter]
    pub fn configured_seeds(&self) -> Vec<Host> {
        self._as
            .configured_seeds
            .iter()
            .map(|host| Host { _as: host.clone() })
            .collect()
    }

    /// Completed cluster tend iterations when the refresh started: ``0`` at
    /// client construction. Informational only.
    #[getter]
    pub fn tend_iteration(&self) -> u64 {
        self._as.tend_iteration
    }

    pub fn __repr__(&self) -> String {
        let seeds: Vec<String> = self.configured_seeds().iter().map(Host::__repr__).collect();
        let cluster_name = match &self._as.cluster_name {
            Some(name) => format!("'{name}'"),
            None => "None".to_string(),
        };
        format!(
            "SeedCandidateContext(cluster_name={cluster_name}, configured_seeds=[{}], \
             tend_iteration={})",
            seeds.join(", "),
            self._as.tend_iteration
        )
    }
}

/// Base class for a seed candidate provider written in Python.
///
/// Subclass it and override :meth:`refresh_seed_candidates`. The client
/// calls it once at construction. When :meth:`needs_periodic_refresh`
/// returns ``True``, the client also calls it every
/// :attr:`ClientPolicy.discovery_refresh_interval` milliseconds on a
/// background thread it starts for the provider, never on the cluster tend
/// task, so a refresh may block, on a cloud API call for instance. The tend
/// task uses the latest successful result; a refresh that takes longer than
/// :attr:`ClientPolicy.discovery_refresh_timeout` is discarded.
///
/// Each client calls the provider from one thread at a time, so it needs no
/// locking of its own; a provider shared by several clients is called by
/// each of them.
///
/// Example::
///
///     class RegistryProvider(SeedCandidateProvider):
///         def __init__(self, registry):
///             self.registry = registry
///
///         def refresh_seed_candidates(self, ctx):
///             return [Host(node.ip, 3000) for node in self.registry.lookup("aerospike")]
///
///         def needs_periodic_refresh(self):
///             return True
///
///     policy = ClientPolicy()
///     policy.seed_candidate_provider = RegistryProvider(registry)
///     client = await new_client(policy, [])
#[gen_stub_pyclass(module = "_native")]
#[pyclass(module = "_native", name = "SeedCandidateProvider", subclass, frozen)]
pub struct PySeedCandidateProvider {}

#[gen_stub_pymethods]
#[pymethods]
impl PySeedCandidateProvider {
    // Accepts and ignores any arguments, so a subclass `__init__` can take
    // its own.
    #[new]
    #[pyo3(signature = (*_args, **_kwargs))]
    pub fn new(_args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) -> Self {
        PySeedCandidateProvider {}
    }

    /// Returns the current seed hosts.
    ///
    /// Args:
    ///     ctx: The call's context.
    ///
    /// Returns:
    ///     The seed hosts, a list of :class:`Host`; possibly empty.
    ///
    /// Raises:
    ///     Exception: Any exception reports a failure. At construction it is
    ///         logged and the constructor's seeds are used; with none, the
    ///         constructor fails with ``ResultCode.SEED_DISCOVERY_FAILED``.
    ///         On a periodic refresh it is logged and the previous seeds
    ///         kept.
    pub fn refresh_seed_candidates(&self, ctx: &SeedCandidateContext) -> PyResult<Vec<Host>> {
        let _ = ctx;
        Err(PyNotImplementedError::new_err(
            "SeedCandidateProvider subclasses must override refresh_seed_candidates",
        ))
    }

    /// Whether the seeds can change and must be refreshed periodically.
    /// Called once per client, at construction. The default, ``False``,
    /// calls :meth:`refresh_seed_candidates` only at construction.
    pub fn needs_periodic_refresh(&self) -> bool {
        false
    }

    /// Releases the provider's resources. Called once per client, after
    /// :meth:`Client.close` and after the last refresh has returned. The
    /// default does nothing.
    pub fn close(&self) {}
}

/// A value of :attr:`ClientPolicy.seed_candidate_provider`.
#[derive(Clone)]
pub enum SeedProvider {
    Static(StaticSeedCandidateProvider),
    Dns(DnsSeedCandidateProvider),
    #[cfg(feature = "discovery-aws")]
    Aws(AwsEc2TagSeedCandidateProvider),
    Python(Arc<Py<PyAny>>),
}

#[cfg(feature = "discovery-aws")]
const SEED_PROVIDER_TYPES: &str = "a StaticSeedCandidateProvider, a DnsSeedCandidateProvider, \
     an AwsEc2TagSeedCandidateProvider or a SeedCandidateProvider subclass";
#[cfg(not(feature = "discovery-aws"))]
const SEED_PROVIDER_TYPES: &str =
    "a StaticSeedCandidateProvider, a DnsSeedCandidateProvider or a SeedCandidateProvider subclass";

impl SeedProvider {
    /// The provider one client uses. A Python provider gets its own adapter
    /// per client, which owns that client's refresh thread.
    pub(crate) fn to_core(
        &self,
        py: Python<'_>,
        refresh_interval: Duration,
        refresh_timeout: Duration,
    ) -> PyResult<SharedSeedCandidateProvider> {
        Ok(match self {
            SeedProvider::Static(p) => p._as.clone(),
            SeedProvider::Dns(p) => p._as.clone(),
            #[cfg(feature = "discovery-aws")]
            SeedProvider::Aws(p) => p._as.clone(),
            SeedProvider::Python(provider) => Arc::new(PythonSeedProvider::new(
                py,
                provider.clone(),
                refresh_interval,
                refresh_timeout,
            )?),
        })
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
        #[cfg(feature = "discovery-aws")]
        if let Ok(p) = obj.extract::<AwsEc2TagSeedCandidateProvider>() {
            return Ok(SeedProvider::Aws(p));
        }
        if obj.is_instance_of::<PySeedCandidateProvider>() {
            return Ok(SeedProvider::Python(Arc::new(obj.to_owned().unbind())));
        }
        Err(PyTypeError::new_err(format!(
            "seed_candidate_provider must be {SEED_PROVIDER_TYPES}"
        )))
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
            #[cfg(feature = "discovery-aws")]
            SeedProvider::Aws(p) => Ok(Bound::new(py, p)?.into_any()),
            SeedProvider::Python(provider) => Ok(provider.bind(py).clone()),
        }
    }
}

impl PyStubType for SeedProvider {
    fn type_output() -> TypeInfo {
        let types = StaticSeedCandidateProvider::type_output() | DnsSeedCandidateProvider::type_output();
        #[cfg(feature = "discovery-aws")]
        let types = types | AwsEc2TagSeedCandidateProvider::type_output();
        types | PySeedCandidateProvider::type_output()
    }
}

/// Runs a Python seed provider for one client.
///
/// A Python call cannot honor the tend task's refresh deadline: it waits for
/// the GIL, and cancelling the future leaves the call running. So the call
/// at construction runs on a blocking thread, later calls on a refresh
/// thread of the adapter's own, and the tend task only reads the latest
/// successful result.
struct PythonSeedProvider {
    shared: Arc<Shared>,
    periodic: bool,
    refresh_interval: Duration,
    refresh_timeout: Duration,
    initialized: AtomicBool,
    refresher: Mutex<Option<Refresher>>,
}

/// What the adapter and its refresh thread share.
struct Shared {
    provider: Arc<Py<PyAny>>,
    name: String,
    seeds: Mutex<Vec<CoreHost>>,
    ctx: Mutex<CoreContext>,
}

/// The refresh thread's handles. Dropping `stop` ends the thread, which
/// then closes the provider and signals `done`.
struct Refresher {
    stop: mpsc::Sender<()>,
    done: mpsc::Receiver<()>,
}

impl PythonSeedProvider {
    fn new(
        py: Python<'_>,
        provider: Arc<Py<PyAny>>,
        refresh_interval: Duration,
        refresh_timeout: Duration,
    ) -> PyResult<Self> {
        let bound = provider.bind(py);
        let periodic = bound
            .call_method0(intern!(py, "needs_periodic_refresh"))?
            .extract::<bool>()?;
        let name = bound.get_type().qualname()?.to_string();
        Ok(PythonSeedProvider {
            shared: Arc::new(Shared {
                provider,
                name,
                seeds: Mutex::new(Vec::new()),
                ctx: Mutex::new(CoreContext::new(None, Vec::new(), 0)),
            }),
            periodic,
            refresh_interval,
            refresh_timeout,
            initialized: AtomicBool::new(false),
            refresher: Mutex::new(None),
        })
    }

    fn start_refresher(&self) {
        let (stop, stop_rx) = mpsc::channel();
        let (done_tx, done) = mpsc::channel();
        let shared = self.shared.clone();
        let (interval, timeout) = (self.refresh_interval, self.refresh_timeout);
        let spawned = thread::Builder::new()
            .name("aerospike-seed-refresh".to_string())
            .spawn(move || {
                shared.refresh_periodically(&stop_rx, interval, timeout);
                let _ = done_tx.send(());
            });
        match spawned {
            Ok(_) => *self.refresher.lock() = Some(Refresher { stop, done }),
            Err(e) => warn!(
                target: "aerospike_native",
                "Seed candidate provider {}: cannot start its refresh thread, so it is \
                 not refreshed: {e}",
                self.shared.name
            ),
        }
    }
}

impl Shared {
    /// Calls the provider; `None` when the interpreter is shutting down.
    fn refresh(&self, ctx: &CoreContext) -> Option<Result<Vec<CoreHost>, String>> {
        Python::try_attach(|py| {
            let ctx = SeedCandidateContext { _as: ctx.clone() };
            let result = self
                .provider
                .bind(py)
                .call_method1(intern!(py, "refresh_seed_candidates"), (ctx,))
                .map_err(|e| e.to_string())?;
            let hosts: Vec<Host> = result.extract().map_err(|_| {
                let type_name = result.get_type().qualname().map(|n| n.to_string());
                format!(
                    "refresh_seed_candidates returned {}, not a list of Host",
                    type_name.unwrap_or_default()
                )
            })?;
            Ok(hosts.into_iter().map(|host| host._as).collect())
        })
    }

    /// The refresh thread's body: refreshes every `interval` until `stop` is
    /// dropped, then closes the provider.
    fn refresh_periodically(
        &self,
        stop: &mpsc::Receiver<()>,
        interval: Duration,
        timeout: Duration,
    ) {
        while let Err(RecvTimeoutError::Timeout) = stop.recv_timeout(interval) {
            let ctx = self.ctx.lock().clone();
            let started = Instant::now();
            match self.refresh(&ctx) {
                None => return,
                Some(Ok(hosts)) if started.elapsed() <= timeout => *self.seeds.lock() = hosts,
                Some(Ok(_)) => warn!(
                    target: "aerospike_native",
                    "Seed candidate provider {} took {} ms, more than the discovery refresh \
                     timeout of {} ms; its result is discarded",
                    self.name,
                    started.elapsed().as_millis(),
                    timeout.as_millis()
                ),
                Some(Err(e)) => warn!(
                    target: "aerospike_native",
                    "Seed candidate provider {} failed; the previous seeds are kept: {e}",
                    self.name
                ),
            }
        }
        self.close();
    }

    fn close(&self) {
        let result = Python::try_attach(|py| {
            self.provider
                .bind(py)
                .call_method0(intern!(py, "close"))
                .map(|_| ())
        });
        if let Some(Err(e)) = result {
            warn!(target: "aerospike_native", "Seed candidate provider {} close failed: {e}", self.name);
        }
    }
}

#[async_trait::async_trait]
impl SeedCandidateProvider for PythonSeedProvider {
    async fn refresh_seed_candidates(&self, ctx: &CoreContext) -> CoreResult<Vec<CoreHost>> {
        if self.initialized.swap(true, Ordering::AcqRel) {
            self.shared.ctx.lock().tend_iteration = ctx.tend_iteration;
            return Ok(self.shared.seeds.lock().clone());
        }
        *self.shared.ctx.lock() = ctx.clone();
        let (shared, call_ctx) = (self.shared.clone(), ctx.clone());
        let result = tokio::task::spawn_blocking(move || shared.refresh(&call_ctx))
            .await
            .map_err(|e| CoreError::client_error(format!("Seed candidate provider failed: {e}")))?
            .unwrap_or_else(|| Err("the Python interpreter is shutting down".to_string()));
        if let Ok(hosts) = &result {
            self.shared.seeds.lock().clone_from(hosts);
        }
        if self.periodic {
            self.start_refresher();
        }
        result.map_err(CoreError::client_error)
    }

    fn needs_periodic_refresh(&self) -> bool {
        self.periodic
    }

    /// `true`: on the tend task the adapter only reads the latest result.
    fn supports_deadline(&self) -> bool {
        true
    }

    async fn close(&self) -> CoreResult<()> {
        let refresher = self.refresher.lock().take();
        let shared = self.shared.clone();
        let timeout = self.refresh_timeout;
        tokio::task::spawn_blocking(move || match refresher {
            Some(Refresher { stop, done }) => {
                drop(stop);
                if done.recv_timeout(timeout).is_err() {
                    warn!(
                        target: "aerospike_native",
                        "Seed candidate provider {} is still refreshing after {} ms; it is \
                         closed once the refresh returns",
                        shared.name,
                        timeout.as_millis()
                    );
                }
            }
            None => shared.close(),
        })
        .await
        .map_err(|e| CoreError::client_error(format!("Seed candidate provider close failed: {e}")))
    }

    fn name(&self) -> String {
        self.shared.name.clone()
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
