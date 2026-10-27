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

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::IoError;
#[cfg(feature = "tls")]
use rustls::pki_types::pem::{self, PemObject};
#[cfg(feature = "tls")]
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};

    ////////////////////////////////////////////////////////////////////////////////////////////
    //
    //  TlsConfig
    //
    ////////////////////////////////////////////////////////////////////////////////////////////

    #[cfg(feature = "tls")]
    #[gen_stub_pyclass(module = "_native")]
    #[pyclass(from_py_object, 
        name = "TlsConfig",
        module = "_native",
        subclass,
    )]
    #[cfg_attr(not(Py_GIL_DISABLED), pyo3(freelist = 100))]
    #[derive(Clone)]
    pub struct TlsConfig {
        pub(crate) _as: std::sync::Arc<rustls::ClientConfig>,
        /// Encrypt only the login exchange; data connections are cleartext.
        pub(crate) for_login_only: bool,
    }

    // Type alias to allow function signatures to compile when TLS is disabled
    #[cfg(not(feature = "tls"))]
    type TlsConfig = ();


    // ----------------------------------------------------------------------
    // Shared construction
    //
    // The parts both constructors share, so each TLS option is implemented
    // once rather than per constructor.
    // ----------------------------------------------------------------------

    /// Build the trust anchors.
    ///
    /// With *cafile*, only the certificates in that file are trusted: a
    /// cluster signed by a private CA must not also accept a certificate from
    /// every public one. Without it, the server is verified against the public
    /// roots compiled into the client, which is what a `tls_name`-only setup
    /// needs. Those roots are fixed at build time; OS certificate updates do
    /// not reach them.
    #[cfg(feature = "tls")]
    fn build_root_store(cafile: Option<&str>) -> PyResult<rustls::RootCertStore> {
        let Some(cafile) = cafile else {
            return Ok(rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.into(),
            });
        };

        let mut root_store = rustls::RootCertStore::empty();
        for cert in load_pem_certs(cafile, "CA file")? {
            root_store.add(cert).map_err(|e| {
                PyErr::new::<IoError, _>(format!("Cannot add CA certificate: {}", e))
            })?;
        }
        // An empty store fails every handshake with an unknown-issuer error
        // that never mentions the file, so refuse it here instead.
        if root_store.is_empty() {
            return Err(PyErr::new::<IoError, _>(format!(
                "No CA certificates found in {}",
                cafile
            )));
        }

        Ok(root_store)
    }

    /// Read every certificate in the PEM file at *path*; *what* names the file
    /// in error messages.
    #[cfg(feature = "tls")]
    fn load_pem_certs(path: &str, what: &str) -> PyResult<Vec<CertificateDer<'static>>> {
        CertificateDer::pem_file_iter(path)
            .map_err(|e| pem_error(e, path, what))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| pem_error(e, path, what))
    }

    /// Read the first PKCS#8 private key in the PEM file at *path*.
    #[cfg(feature = "tls")]
    fn load_pem_pkcs8_key(path: &str) -> PyResult<PrivatePkcs8KeyDer<'static>> {
        PrivatePkcs8KeyDer::from_pem_file(path).map_err(|e| match e {
            pem::Error::NoItemsFound => {
                PyErr::new::<IoError, _>(format!("No private key found in {}", path))
            }
            e => pem_error(e, path, "key file"),
        })
    }

    /// Convert a PEM read failure on the file at *path* into an `IoError`.
    #[cfg(feature = "tls")]
    fn pem_error(e: pem::Error, path: &str, what: &str) -> PyErr {
        let message = match e {
            pem::Error::Io(e) => format!("Cannot open {} {}: {}", what, path, e),
            e => format!("Cannot parse {} {}: {}", what, path, e),
        };
        PyErr::new::<IoError, _>(message)
    }

    /// Resolve protocol-version names to rustls versions.
    ///
    /// Unknown names are rejected rather than ignored: silently dropping a
    /// version restriction is how a caller ends up negotiating something they
    /// explicitly excluded.
    #[cfg(feature = "tls")]
    fn resolve_protocols(
        protocols: &[String],
    ) -> PyResult<Vec<&'static rustls::SupportedProtocolVersion>> {
        protocols
            .iter()
            .map(|name| match name.trim().to_ascii_uppercase().as_str() {
                "TLSV1.2" | "TLSV12" | "TLS1.2" => Ok(&rustls::version::TLS12),
                "TLSV1.3" | "TLSV13" | "TLS1.3" => Ok(&rustls::version::TLS13),
                other => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Unsupported TLS protocol {:?}; supported: TLSv1.2, TLSv1.3",
                    other
                ))),
            })
            .collect()
    }

    /// Resolve cipher-suite names against the default provider's suites.
    ///
    /// Rejects unknown names for the same reason as `resolve_protocols`, and
    /// names the available suites in the error so the caller can correct it
    /// without going to the rustls docs.
    #[cfg(feature = "tls")]
    fn resolve_cipher_suites(
        ciphers: &[String],
    ) -> PyResult<Vec<rustls::SupportedCipherSuite>> {
        let provider = rustls::crypto::CryptoProvider::get_default()
            .cloned()
            .unwrap_or_else(|| std::sync::Arc::new(rustls::crypto::ring::default_provider()));
        let available: Vec<rustls::SupportedCipherSuite> = provider.cipher_suites.clone();
        ciphers
            .iter()
            .map(|name| {
                let want = name.trim().to_ascii_uppercase();
                available
                    .iter()
                    .find(|suite: &&rustls::SupportedCipherSuite| {
                        format!("{:?}", suite.suite()).to_ascii_uppercase() == want
                    })
                    .copied()
                    .ok_or_else(|| {
                        let names: Vec<String> = available
                            .iter()
                            .map(|s| format!("{:?}", s.suite()))
                            .collect();
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Unknown cipher suite {:?}; supported: {}",
                            name,
                            names.join(", ")
                        ))
                    })
            })
            .collect()
    }

    /// Assemble a `ClientConfig` from resolved parts.
    #[cfg(feature = "tls")]
    fn build_client_config(
        root_store: rustls::RootCertStore,
        protocols: Option<&[String]>,
        ciphers: Option<&[String]>,
        client_auth: Option<(
            Vec<rustls::pki_types::CertificateDer<'static>>,
            rustls::pki_types::PrivateKeyDer<'static>,
        )>,
    ) -> PyResult<rustls::ClientConfig> {
        use rustls::ClientConfig;

        let versions = match protocols {
            Some(names) => resolve_protocols(names)?,
            None => rustls::ALL_VERSIONS.to_vec(),
        };

        let builder = match ciphers {
            Some(names) => {
                let suites = resolve_cipher_suites(names)?;
                let base = rustls::crypto::CryptoProvider::get_default()
                    .cloned()
                    .unwrap_or_else(|| {
                        std::sync::Arc::new(rustls::crypto::ring::default_provider())
                    });
                let provider = rustls::crypto::CryptoProvider {
                    cipher_suites: suites,
                    ..(*base).clone()
                };
                ClientConfig::builder_with_provider(provider.into())
                    .with_protocol_versions(&versions)
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Cannot build TLS config: {}",
                            e
                        ))
                    })?
            }
            None => ClientConfig::builder_with_protocol_versions(&versions),
        };

        let builder = builder.with_root_certificates(root_store);

        match client_auth {
            Some((certs, key)) => builder.with_client_auth_cert(certs, key).map_err(|e| {
                PyErr::new::<IoError, _>(format!(
                    "Cannot build TLS config with client auth: {}",
                    e
                ))
            }),
            None => Ok(builder.with_no_client_auth()),
        }
    }

    #[cfg(feature = "tls")]
    #[gen_stub_pymethods]
    #[pymethods]
    impl TlsConfig {
        /// Create a TlsConfig.
        ///
        /// Args:
        ///     cafile: Path to a CA certificate file (PEM). When given, only
        ///         the CAs in this file are trusted. Optional -- when omitted
        ///         the server is verified against the public roots compiled
        ///         into the client (not the OS trust store), which is what a
        ///         ``tls_name``-only setup needs.
        ///     protocols: Allowed TLS versions, e.g. ``["TLSv1.3"]``. Omit for
        ///         the rustls default (TLS 1.2 and 1.3).
        ///     ciphers: Allowed cipher suites by rustls name, e.g.
        ///         ``["TLS13_AES_256_GCM_SHA384"]``. Omit for the provider
        ///         default. Unknown names raise rather than being ignored.
        ///     for_login_only: Use TLS for the login exchange only; every
        ///         other connection to the server is cleartext. The login
        ///         connection is closed once the session token is held and
        ///         a new cleartext connection is opened to the node's
        ///         cleartext address, so no socket is ever downgraded.
        ///         Requires authentication; the client refuses it with no
        ///         credentials, where it would only mean "no TLS at all".
        ///         Default ``False``: TLS for all communication.
        ///
        /// Returns:
        ///     TlsConfig
        #[new]
        #[pyo3(signature = (cafile=None, *, protocols=None, ciphers=None, for_login_only=false))]
        pub fn new(
            cafile: Option<String>,
            protocols: Option<Vec<String>>,
            ciphers: Option<Vec<String>>,
            for_login_only: bool,
        ) -> PyResult<Self> {
            let root_store = build_root_store(cafile.as_deref())?;
            let config = build_client_config(
                root_store,
                protocols.as_deref(),
                ciphers.as_deref(),
                None,
            )?;
            Ok(TlsConfig { _as: std::sync::Arc::new(config), for_login_only })
        }

        /// Whether TLS covers the login exchange only; see the constructor.
        #[getter]
        pub fn get_for_login_only(&self) -> bool {
            self.for_login_only
        }

        #[setter]
        pub fn set_for_login_only(&mut self, value: bool) {
            self.for_login_only = value;
        }

        /// Create a TlsConfig with client (mutual) authentication.
        ///
        /// Args:
        ///     cafile: Path to a CA certificate file (PEM), or ``None`` to
        ///         verify against the public roots compiled into the client;
        ///         see :meth:`TlsConfig`.
        ///     certfile: Path to the client certificate file (PEM).
        ///     keyfile: Path to the client private key file (PEM, PKCS#8).
        ///     protocols: Allowed TLS versions; see :meth:`TlsConfig`.
        ///     ciphers: Allowed cipher suites; see :meth:`TlsConfig`.
        ///     for_login_only: TLS for the login exchange only; see
        ///         :meth:`TlsConfig`.
        ///
        /// Returns:
        ///     TlsConfig
        #[staticmethod]
        #[pyo3(signature = (cafile, certfile, keyfile, *, protocols=None, ciphers=None, for_login_only=false))]
        pub fn with_client_auth(
            cafile: Option<String>,
            certfile: String,
            keyfile: String,
            protocols: Option<Vec<String>>,
            ciphers: Option<Vec<String>>,
            for_login_only: bool,
        ) -> PyResult<Self> {
            let root_store = build_root_store(cafile.as_deref())?;
            let client_certs = load_pem_certs(&certfile, "client cert file")?;
            let client_key = load_pem_pkcs8_key(&keyfile)?;

            let config = build_client_config(
                root_store,
                protocols.as_deref(),
                ciphers.as_deref(),
                Some((client_certs, client_key.into())),
            )?;
            Ok(TlsConfig { _as: std::sync::Arc::new(config), for_login_only })
        }
    }

    // These never unwrap an `Err` or format a `PyErr`: its `Debug` needs a
    // live interpreter, which this test binary does not start.
    #[cfg(all(test, feature = "tls"))]
    mod tests {
        use super::{build_root_store, load_pem_pkcs8_key};

        const TEST_CA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/test-ca.pem");

        #[test]
        fn cafile_is_the_only_trust_anchor() {
            let Ok(store) = build_root_store(Some(TEST_CA)) else {
                panic!("test CA must load");
            };
            assert_eq!(store.len(), 1);
        }

        #[test]
        fn bundled_roots_load_without_cafile() {
            let Ok(store) = build_root_store(None) else {
                panic!("bundled roots must load");
            };
            assert!(!store.is_empty());
        }

        #[test]
        fn cafile_without_certificates_is_rejected() {
            let path = std::env::temp_dir()
                .join(format!("pnc-empty-ca-{}.pem", std::process::id()));
            std::fs::write(&path, "").expect("write empty CA file");
            let result = build_root_store(path.to_str());
            let _ = std::fs::remove_file(&path);
            assert!(result.is_err());
        }

        #[test]
        fn key_file_without_a_key_is_rejected() {
            assert!(load_pem_pkcs8_key(TEST_CA).is_err());
        }
    }
