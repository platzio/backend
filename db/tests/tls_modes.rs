//! End-to-end verification of the PostgreSQL TLS modes.
//!
//! Spins up a TLS-enabled PostgreSQL via `testcontainers`, minting its own
//! CA -> server certificate chain with `rcgen`, and drives every `PGSSLMODE`
//! through the crate's real connector ([`platz_db::tls::build_connector`] +
//! [`platz_db::tls::pg_ssl_mode`]). For each mode it reads `pg_stat_ssl` to
//! confirm whether the session is actually encrypted, and checks that
//! `verify-full` accepts a trusted cert while rejecting an untrusted one.
//!
//! Self-contained: `cargo test` runs it with no external setup. It needs a
//! reachable Docker daemon (to start the container) and pulls
//! `postgres:16-alpine`; when Docker is unavailable the test skips cleanly
//! instead of failing.

use std::time::Duration;

use platz_db::{SslMode, SslSettings, tls};
use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{CopyTargetOptions, GenericImage, ImageExt};
use tokio::time::sleep;

/// Runs as the container entrypoint: copies the injected certs to a location
/// the postgres user can read (key must be 0600 and owned by `postgres`), then
/// hands off to the stock entrypoint with TLS enabled.
const ENTRYPOINT_SCRIPT: &str = "set -e
mkdir -p /etc/pg-certs
cp /certs/server.crt /certs/server.key /etc/pg-certs/
chown postgres:postgres /etc/pg-certs/server.crt /etc/pg-certs/server.key
chmod 600 /etc/pg-certs/server.key
exec docker-entrypoint.sh postgres \
  -c ssl=on \
  -c ssl_cert_file=/etc/pg-certs/server.crt \
  -c ssl_key_file=/etc/pg-certs/server.key";

struct TestCerts {
    ca_pem: String,
    server_cert_pem: String,
    server_key_pem: String,
}

/// Mints a CA and a `localhost` server certificate signed by it. The server
/// cert carries a SAN and the serverAuth EKU so it passes `verify-full`.
fn mint_certs() -> TestCerts {
    let ca_key = KeyPair::generate().expect("generate CA key");
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).expect("CA params");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "platz-test-ca");
    let ca_cert = ca_params.self_signed(&ca_key).expect("self-sign CA");

    let server_key = KeyPair::generate().expect("generate server key");
    let mut server_params =
        CertificateParams::new(vec!["localhost".to_string()]).expect("server params");
    server_params
        .distinguished_name
        .push(DnType::CommonName, "localhost");
    server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    let issuer = Issuer::new(ca_params, ca_key);
    let server_cert = server_params
        .signed_by(&server_key, &issuer)
        .expect("sign server cert");

    TestCerts {
        ca_pem: ca_cert.pem(),
        server_cert_pem: server_cert.pem(),
        server_key_pem: server_key.serialize_pem(),
    }
}

/// Connects with the given settings through the crate's connector and reports
/// whether the resulting session is encrypted, per `pg_stat_ssl`.
async fn negotiated_ssl(port: u16, settings: &SslSettings) -> Result<bool, String> {
    let connector = tls::build_connector(settings).map_err(|e| e.to_string())?;
    let base = format!("host=localhost port={port} user=postgres dbname=platz");
    let client = match connector {
        None => {
            let (client, conn) = tokio_postgres::connect(&base, tokio_postgres::NoTls)
                .await
                .map_err(|e| e.to_string())?;
            tokio::spawn(async move {
                let _ = conn.await;
            });
            client
        }
        Some(connector) => {
            let mut config: tokio_postgres::Config = base
                .parse()
                .map_err(|e: tokio_postgres::Error| e.to_string())?;
            config.ssl_mode(tls::pg_ssl_mode(settings.mode));
            let (client, conn) = config.connect(connector).await.map_err(|e| e.to_string())?;
            tokio::spawn(async move {
                let _ = conn.await;
            });
            client
        }
    };
    let row = client
        .query_one(
            "SELECT coalesce(ssl, false) FROM pg_stat_ssl WHERE pid = pg_backend_pid()",
            &[],
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.get::<_, bool>(0))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tls_modes_end_to_end() {
    let certs = mint_certs();

    let image = GenericImage::new("postgres", "16-alpine")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_entrypoint("sh")
        .with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
        .with_env_var("POSTGRES_DB", "platz")
        .with_env_var("POSTGRES_USER", "postgres")
        .with_copy_to(
            CopyTargetOptions::new("/certs/server.crt"),
            certs.server_cert_pem.into_bytes(),
        )
        .with_copy_to(
            CopyTargetOptions::new("/certs/server.key"),
            certs.server_key_pem.into_bytes(),
        )
        .with_cmd(["-c", ENTRYPOINT_SCRIPT]);

    let container = match image.start().await {
        Ok(container) => container,
        Err(err) => {
            eprintln!("skipping tls_modes_end_to_end: Docker not available ({err})");
            return;
        }
    };
    let port = container
        .get_host_port_ipv4(5432.tcp())
        .await
        .expect("mapped host port");

    // Trust anchor for verify-full: the CA we minted, written to a file that
    // PGSSLROOTCERT (root_cert) points at.
    let ca_file = tempfile::NamedTempFile::new().expect("temp CA file");
    std::fs::write(ca_file.path(), &certs.ca_pem).expect("write CA PEM");
    let ca_path = ca_file.path().to_str().expect("utf-8 CA path").to_string();

    let require = SslSettings {
        mode: SslMode::Require,
        root_cert: None,
    };

    // The wait strategy fires on the first "ready" line, which is the temporary
    // init server. Poll over TCP until the real TLS server is accepting.
    let mut ready = false;
    for _ in 0..60 {
        if negotiated_ssl(port, &require).await.is_ok() {
            ready = true;
            break;
        }
        sleep(Duration::from_secs(1)).await;
    }
    assert!(ready, "postgres did not become ready in time");

    // disable -> plaintext session.
    let disable = SslSettings {
        mode: SslMode::Disable,
        root_cert: None,
    };
    assert_eq!(
        negotiated_ssl(port, &disable).await,
        Ok(false),
        "disable must produce a plaintext session"
    );

    // prefer -> TLS, since the server offers it.
    let prefer = SslSettings {
        mode: SslMode::Prefer,
        root_cert: None,
    };
    assert_eq!(
        negotiated_ssl(port, &prefer).await,
        Ok(true),
        "prefer must negotiate TLS when the server offers it"
    );

    // require -> TLS.
    assert_eq!(
        negotiated_ssl(port, &require).await,
        Ok(true),
        "require must use TLS"
    );

    // verify-full with the CA trusted -> TLS, chain + hostname verified.
    let verify_full_trusted = SslSettings {
        mode: SslMode::VerifyFull,
        root_cert: Some(ca_path.clone()),
    };
    assert_eq!(
        negotiated_ssl(port, &verify_full_trusted).await,
        Ok(true),
        "verify-full must succeed when the server CA is trusted"
    );

    // verify-full without our CA (system trust store) -> rejected.
    let verify_full_untrusted = SslSettings {
        mode: SslMode::VerifyFull,
        root_cert: None,
    };
    assert!(
        negotiated_ssl(port, &verify_full_untrusted).await.is_err(),
        "verify-full must reject a certificate signed by an untrusted CA"
    );

    // Finally, exercise the actual diesel-async pool setup path (the same one
    // the pool uses via ManagerConfig::custom_setup) over verified TLS.
    let connector = tls::build_connector(&verify_full_trusted).expect("build connector");
    let url = format!("host=localhost port={port} user=postgres dbname=platz");
    assert!(
        tls::establish_connection(&url, connector, SslMode::VerifyFull)
            .await
            .is_ok(),
        "diesel-async pool setup should connect with verify-full + trusted CA"
    );
}
