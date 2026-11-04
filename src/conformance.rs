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

//! Hooks for the shared discovery conformance runner.
//!
//! The `aerospike-client-conformance` fixtures drive address translation,
//! the seed refresh cycle and certificate name matching through the
//! client's own code, without a cluster. These reach core's seams for
//! them. Private: kept for the runner, not API.

use std::sync::Arc;

use aerospike_core::discovery::conformance::{self, Refresher};
use aerospike_core::discovery::{
    Endpoint, SeedCandidateContext, SeedCandidateProvider, SourceList,
};
use aerospike_core::{Error as CoreError, Host as CoreHost, Result as CoreResult};
use parking_lot::Mutex;
use pyo3::exceptions::{PyBaseException, PyValueError};
use pyo3::prelude::*;
#[cfg(feature = "tls")]
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use crate::blocking::run_blocking;
use crate::discovery::SeedProvider;
use crate::errors::RustClientError;
use crate::{ClientPolicy, Host};

/// What one refresh produced: the provider's hosts or its error, and the
/// seed list in effect afterwards.
type Outcome = (Option<Vec<Host>>, Option<Py<PyBaseException>>, Vec<Host>);

/// Translates an advertised address with the translator and miss mode a
/// client built from `policy` uses.
///
/// Raises the client's translation-miss error on a strict miss.
#[gen_stub_pyfunction(module = "_native")]
#[pyfunction]
#[pyo3(
    name = "_conformance_translate",
    signature = (policy, node_name, host, port, tls_name = None, alternate = false)
)]
pub(crate) fn translate(
    policy: &ClientPolicy,
    node_name: String,
    host: String,
    port: u16,
    tls_name: Option<String>,
    alternate: bool,
) -> PyResult<Host> {
    let source_list = if alternate {
        SourceList::Alternate
    } else {
        SourceList::Standard
    };
    let advertised = Endpoint::new(node_name, tls_name, host, port, source_list);
    conformance::translate_with_policy(&policy._as, &advertised)
        .map(|host| Host { _as: host })
        .map_err(|e| RustClientError(e).into())
}

/// Whether the PEM `certificate`'s subject alternative names match
/// `tls_name`, as the TLS handshake checks them. A `tls_name` that is not a
/// DNS name or an IP address matches nothing.
#[cfg(feature = "tls")]
#[gen_stub_pyfunction(module = "_native")]
#[pyfunction]
#[pyo3(name = "_conformance_tls_name_matches")]
pub(crate) fn tls_name_matches(certificate: &Bound<'_, PyBytes>, tls_name: &str) -> PyResult<bool> {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, ServerName};

    let der = CertificateDer::from_pem_slice(certificate.as_bytes())
        .map_err(|e| PyValueError::new_err(format!("certificate is not PEM: {e}")))?;
    let parsed = rustls::server::ParsedCertificate::try_from(&der)
        .map_err(|e| PyValueError::new_err(format!("certificate does not parse: {e}")))?;
    Ok(match ServerName::try_from(tls_name) {
        Ok(name) => rustls::client::verify_server_name(&parsed, &name).is_ok(),
        Err(_) => false,
    })
}

/// Drives a seed candidate provider through the client's own refresh and
/// merge code, one refresh per call, with the policy's merge policy.
///
/// :meth:`initial` is the refresh at client initialization, each
/// :meth:`refresh` a periodic one.
#[gen_stub_pyclass]
#[pyclass(module = "_native", name = "_ConformanceRefresher")]
pub(crate) struct ConformanceRefresher {
    refresher: Mutex<Refresher>,
}

impl ConformanceRefresher {
    fn over(
        policy: &aerospike_core::ClientPolicy,
        provider: Arc<dyn SeedCandidateProvider>,
        static_seeds: Vec<Host>,
    ) -> Self {
        let static_seeds = static_seeds.into_iter().map(|host| host._as).collect();
        ConformanceRefresher {
            refresher: Mutex::new(Refresher::new(policy, provider, static_seeds)),
        }
    }

    fn outcome(
        py: Python<'_>,
        (result, seeds): (CoreResult<Vec<CoreHost>>, Vec<CoreHost>),
    ) -> Outcome {
        let seeds = hosts(seeds);
        match result {
            Ok(found) => (Some(hosts(found)), None, seeds),
            Err(e) => (
                None,
                Some(PyErr::from(RustClientError(e)).into_value(py)),
                seeds,
            ),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl ConformanceRefresher {
    /// A refresher over `policy.seed_candidate_provider`.
    ///
    /// Raises ``ValueError`` when the policy has no provider, or has a
    /// ``SeedCandidateProvider`` subclass: that answers periodic refreshes
    /// from its own thread, so it cannot be driven one refresh at a time.
    #[new]
    fn new(py: Python<'_>, policy: &ClientPolicy, static_seeds: Vec<Host>) -> PyResult<Self> {
        if let Some(SeedProvider::Python(_)) = policy.seed_candidate_provider {
            return Err(PyValueError::new_err(
                "a SeedCandidateProvider subclass refreshes on its own thread, so it cannot be \
                 driven one refresh at a time",
            ));
        }
        let core = policy.core_policy(py)?;
        let provider = core
            .seed_candidate_provider
            .clone()
            .ok_or_else(|| PyValueError::new_err("the policy has no seed_candidate_provider"))?;
        Ok(Self::over(&core, provider, static_seeds))
    }

    /// A refresher over a provider that returns `results` in order, one per
    /// refresh, starting with :meth:`initial`.
    #[staticmethod]
    fn scripted(policy: &ClientPolicy, results: Vec<Vec<Host>>, static_seeds: Vec<Host>) -> Self {
        let results = results
            .into_iter()
            .map(|r| r.into_iter().map(|h| h._as).collect());
        let provider = Arc::new(Scripted {
            results: Mutex::new(results.collect::<Vec<_>>().into_iter()),
        });
        Self::over(&policy._as, provider, static_seeds)
    }

    /// The refresh at client initialization: the provider's hosts or error,
    /// and the seed list it establishes.
    fn initial(&mut self, py: Python<'_>) -> PyResult<Outcome> {
        let refresher = self.refresher.get_mut();
        let refreshed = run_blocking(py, async { Ok(refresher.initial().await) })?;
        Ok(Self::outcome(py, refreshed))
    }

    /// One periodic refresh: the provider's hosts or error, and the seed
    /// list in effect afterwards, the previous one when the refresh
    /// published nothing.
    fn refresh(&mut self, py: Python<'_>) -> PyResult<Outcome> {
        let refresher = self.refresher.get_mut();
        let refreshed = run_blocking(py, async { Ok(refresher.refresh().await) })?;
        Ok(Self::outcome(py, refreshed))
    }

    /// The seed list in effect.
    #[getter]
    fn seeds(&self) -> Vec<Host> {
        hosts(self.refresher.lock().seeds().to_vec())
    }
}

fn hosts(hosts: Vec<CoreHost>) -> Vec<Host> {
    hosts.into_iter().map(|host| Host { _as: host }).collect()
}

/// Returns its results in order, one per refresh.
struct Scripted {
    results: Mutex<std::vec::IntoIter<Vec<CoreHost>>>,
}

#[async_trait::async_trait]
impl SeedCandidateProvider for Scripted {
    async fn refresh_seed_candidates(
        &self,
        _ctx: &SeedCandidateContext,
    ) -> CoreResult<Vec<CoreHost>> {
        self.results
            .lock()
            .next()
            .ok_or_else(|| CoreError::client_error("the scripted results are used up"))
    }

    fn needs_periodic_refresh(&self) -> bool {
        true
    }

    fn supports_deadline(&self) -> bool {
        true
    }
}
