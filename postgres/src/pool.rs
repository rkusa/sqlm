use std::str::FromStr as _;
use std::sync::Arc;

use deadpool_postgres::{Manager, ManagerConfig, RecyclingMethod};
use tokio_postgres::NoTls;
use tokio_postgres::config::SslMode;

use crate::error::ErrorKind;
use crate::{Error, Session};

/// Run before handing out a pooled connection again. Besides checking that the connection is still
/// alive, it rejects connections that turned read-only, e.g. because their server was demoted to
/// a standby during a failover. The server keeps the existing connections open in that case, so
/// without this check, they would keep failing every write until the process is restarted.
const RECYCLE_CHECK_SQL: &str = "\
    DO $$ BEGIN \
        IF pg_is_in_recovery() OR current_setting('transaction_read_only')::bool THEN \
            RAISE EXCEPTION 'connection is read-only'; \
        END IF; \
    END $$";

#[derive(Clone)]
pub struct Pool(deadpool_postgres::Pool);

impl Pool {
    pub fn new(database_url: &str, pool_size: usize) -> Result<Self, Error> {
        let mut config = tokio_postgres::Config::from_str(database_url)?;
        if config.get_application_name().is_none() {
            config.application_name(env!("CARGO_PKG_NAME"));
        }

        let manager_config = || ManagerConfig {
            recycling_method: RecyclingMethod::Custom(RECYCLE_CHECK_SQL.to_string()),
        };

        // TODO: take all possible SSL variants into account, see e.g.
        // https://github.com/jbg/tokio-postgres-rustls/issues/11
        let mgr = match config.get_ssl_mode() {
            SslMode::Disable => Manager::from_config(config, NoTls, manager_config()),
            _ => Manager::from_config(
                config,
                tokio_postgres_rustls::MakeRustlsConnect::new(tls_config()),
                manager_config(),
            ),
        };
        let pool = deadpool_postgres::Pool::builder(mgr)
            .max_size(pool_size)
            .build()?;
        Ok(Self(pool))
    }

    pub fn from_env(pool_size: usize) -> Result<Self, Error> {
        Self::new(
            dotenvy::var("DATABASE_URL")
                .map_err(|_| ErrorKind::MissingDatabaseUrlEnv)?
                .as_str(),
            pool_size,
        )
    }

    #[tracing::instrument(skip_all)]
    pub async fn connect(&self) -> Result<Session, Error> {
        let conn = self.0.get().await?;
        Ok(Session(conn))
    }
}

/// A TLS config that encrypts the connection but does not verify the server certificate. Uses the
/// process-wide default crypto provider if one is installed and falls back to `ring` otherwise.
fn tls_config() -> rustls::ClientConfig {
    let crypto_provider = rustls::crypto::CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()));
    rustls::ClientConfig::builder_with_provider(Arc::clone(&crypto_provider))
        .with_safe_default_protocol_versions()
        .expect("crypto provider supports the default TLS protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoServerCertVerify { crypto_provider }))
        .with_no_client_auth()
}

#[derive(Debug)]
struct NoServerCertVerify {
    crypto_provider: Arc<rustls::crypto::CryptoProvider>,
}

impl rustls::client::danger::ServerCertVerifier for NoServerCertVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.crypto_provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.crypto_provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.crypto_provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}
