use std::str::FromStr as _;
use std::sync::Arc;

use deadpool_postgres::{Manager, ManagerConfig, RecyclingMethod};
use tokio_postgres::NoTls;
use tokio_postgres::config::SslMode;

use crate::error::ErrorKind;
use crate::{Error, Session};

#[derive(Clone)]
pub struct Pool(deadpool_postgres::Pool);

impl Pool {
    pub fn new(database_url: &str, pool_size: usize) -> Result<Self, Error> {
        let mut config = tokio_postgres::Config::from_str(database_url)?;
        config.application_name(env!("CARGO_PKG_NAME"));

        // TODO: take all possible SSL variants into account, see e.g.
        // https://github.com/jbg/tokio-postgres-rustls/issues/11
        let mgr = match config.get_ssl_mode() {
            SslMode::Disable => Manager::from_config(
                config,
                NoTls,
                ManagerConfig {
                    recycling_method: RecyclingMethod::Fast,
                },
            ),
            _ => Manager::from_config(
                config,
                {
                    let config = rustls::ClientConfig::builder()
                        .dangerous()
                        .with_custom_certificate_verifier(Arc::new(NoServerCertVerify::default()))
                        .with_no_client_auth();
                    tokio_postgres_rustls::MakeRustlsConnect::new(config)
                },
                ManagerConfig {
                    recycling_method: RecyclingMethod::Fast,
                },
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
        // Don't trace connect, as this would create an endless loop of connecting again and
        // again when persisting the connect trace!
        let conn = self.0.get().await?;
        Ok(Session(conn))
    }
}

#[derive(Debug)]
struct NoServerCertVerify {
    crypto_provider: Arc<rustls::crypto::CryptoProvider>,
}

impl Default for NoServerCertVerify {
    fn default() -> Self {
        Self {
            crypto_provider: Arc::clone(
                rustls::crypto::CryptoProvider::get_default()
                    .expect("no default provider for rustls installed"),
            ),
        }
    }
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
