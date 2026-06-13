//! TLS support for PostgreSQL connections.
//!
//! Platz assembles the PostgreSQL connection itself and does not link libpq,
//! so libpq-style controls (`PGSSLMODE`, `PGSSLROOTCERT`, …) are interpreted
//! here and turned into a `rustls`-backed TLS connector that is shared by both
//! the `diesel-async` connection pool and the dedicated `LISTEN`/`NOTIFY`
//! connection in [`crate::events`].

use crate::config::{SslMode, SslSettings};
use diesel::{ConnectionError, ConnectionResult};
use diesel_async::AsyncPgConnection;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio_postgres::NoTls;
use tokio_postgres::config::SslMode as PgSslMode;
use tokio_postgres_rustls::MakeRustlsConnect;

/// Errors that can occur while building the TLS connector.
#[derive(Debug, thiserror::Error)]
pub enum TlsError {
    #[error("Invalid TLS configuration: {0}")]
    Config(String),
    #[error("Failed reading CA bundle {path:?}: {source}")]
    ReadCaBundle {
        path: String,
        source: std::io::Error,
    },
    #[error("CA bundle {0:?} contained no certificates")]
    EmptyCaBundle(String),
    #[error("Failed loading certificates: {0}")]
    Rustls(#[from] rustls::Error),
}

/// The TLS mode mapped to the [`PgSslMode`] understood by `tokio-postgres`.
///
/// `tokio-postgres` only uses this to decide *whether* to attempt or require
/// TLS during negotiation; certificate verification is the connector's job and
/// is handled by the [`ClientConfig`] we build in [`build_connector`].
pub fn pg_ssl_mode(mode: SslMode) -> PgSslMode {
    match mode {
        SslMode::Disable => PgSslMode::Disable,
        SslMode::Prefer => PgSslMode::Prefer,
        SslMode::Require | SslMode::VerifyFull => PgSslMode::Require,
    }
}

/// Builds the rustls-based TLS connector for the given settings.
///
/// Returns `Ok(None)` when TLS is disabled, in which case callers should use
/// [`tokio_postgres::NoTls`] and the plaintext code path.
pub fn build_connector(settings: &SslSettings) -> Result<Option<MakeRustlsConnect>, TlsError> {
    let config = match settings.mode {
        SslMode::Disable => return Ok(None),
        // prefer/require encrypt the connection but do not verify the server
        // certificate, matching libpq's behavior for these modes.
        SslMode::Prefer | SslMode::Require => client_config_builder()?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCert))
            .with_no_client_auth(),
        // verify-full validates the certificate chain and the hostname.
        SslMode::VerifyFull => client_config_builder()?
            .with_root_certificates(load_root_store(settings.root_cert.as_deref())?)
            .with_no_client_auth(),
    };
    Ok(Some(MakeRustlsConnect::new(config)))
}

/// Starts a rustls [`ClientConfig`] builder using an explicit crypto provider.
///
/// Using `builder_with_provider` avoids depending on a process-wide default
/// provider being installed, which the rest of the workspace does not do.
fn client_config_builder()
-> Result<rustls::ConfigBuilder<rustls::ClientConfig, rustls::WantsVerifier>, TlsError> {
    ClientConfig::builder_with_provider(Arc::new(rustls::crypto::aws_lc_rs::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|err| TlsError::Config(err.to_string()))
}

/// Loads the trust anchors for `verify-full`. When a CA bundle path is given it
/// is used exclusively; otherwise the operating system trust store is used.
fn load_root_store(root_cert: Option<&str>) -> Result<RootCertStore, TlsError> {
    let mut roots = RootCertStore::empty();

    match root_cert {
        Some(path) => {
            let file = File::open(path).map_err(|source| TlsError::ReadCaBundle {
                path: path.to_string(),
                source,
            })?;
            let mut reader = BufReader::new(file);
            let mut added = 0usize;
            for cert in rustls_pemfile::certs(&mut reader) {
                let cert = cert.map_err(|source| TlsError::ReadCaBundle {
                    path: path.to_string(),
                    source,
                })?;
                roots.add(cert)?;
                added += 1;
            }
            if added == 0 {
                return Err(TlsError::EmptyCaBundle(path.to_string()));
            }
        }
        None => {
            let result = rustls_native_certs::load_native_certs();
            for cert in result.certs {
                // Skip certificates the trust store can't parse rather than
                // failing the whole connection over one bad entry.
                roots.add(cert).ok();
            }
            if roots.is_empty() {
                return Err(TlsError::Config(
                    "No usable certificates found in the system trust store; \
                     set PGSSLROOTCERT to a CA bundle"
                        .to_string(),
                ));
            }
        }
    }

    Ok(roots)
}

/// Establishes a single `AsyncPgConnection`, applying the configured TLS mode.
///
/// This is wired into the `diesel-async` pool via `ManagerConfig::custom_setup`
/// so that every pooled connection negotiates TLS identically.
pub async fn establish_connection(
    url: &str,
    connector: Option<MakeRustlsConnect>,
    mode: SslMode,
) -> ConnectionResult<AsyncPgConnection> {
    match connector {
        None => {
            let (client, conn) = tokio_postgres::connect(url, NoTls)
                .await
                .map_err(|err| ConnectionError::BadConnection(err.to_string()))?;
            AsyncPgConnection::try_from_client_and_connection(client, conn).await
        }
        Some(connector) => {
            let mut config: tokio_postgres::Config =
                url.parse().map_err(|err: tokio_postgres::Error| {
                    ConnectionError::BadConnection(err.to_string())
                })?;
            config.ssl_mode(pg_ssl_mode(mode));
            let (client, conn) = config
                .connect(connector)
                .await
                .map_err(|err| ConnectionError::BadConnection(err.to_string()))?;
            AsyncPgConnection::try_from_client_and_connection(client, conn).await
        }
    }
}

/// A certificate verifier that accepts any server certificate.
///
/// Used for the `prefer` and `require` modes, which encrypt the connection but
/// — like libpq — do not authenticate the server. Use `verify-full` when the
/// server's identity must be verified.
#[derive(Debug)]
struct AcceptAnyServerCert;

impl ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}
