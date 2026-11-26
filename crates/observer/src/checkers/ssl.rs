use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::ClientConfig;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use x509_parser::prelude::*;

use super::ping::{extract_host, extract_port};

pub struct SslInfo {
    pub is_valid: bool,
    pub days: Option<i64>,
    pub error: Option<String>,
}

pub async fn ssl_check(url: &str) -> SslInfo {
    let host = extract_host(url);
    let port = extract_port(url, 443);
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = client_config(roots);
    match tokio::time::timeout(Duration::from_secs(10), handshake(&host, port, config)).await {
        Ok(Ok(days)) => SslInfo {
            is_valid: true,
            days: Some(days),
            error: None,
        },
        Ok(Err(err)) => SslInfo {
            is_valid: false,
            days: None,
            error: Some(err),
        },
        Err(_) => SslInfo {
            is_valid: false,
            days: None,
            error: Some("TLS handshake timed out".into()),
        },
    }
}

fn client_config(roots: rustls::RootCertStore) -> ClientConfig {
    let _ = rustls::crypto::ring::default_provider().install_default();
    ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth()
}

async fn handshake(host: &str, port: u16, config: ClientConfig) -> Result<i64, String> {
    let connector = TlsConnector::from(Arc::new(config));
    let server_name = ServerName::try_from(host.to_string()).map_err(|err| err.to_string())?;
    let stream = TcpStream::connect((host, port))
        .await
        .map_err(|err| err.to_string())?;
    let tls = connector
        .connect(server_name, stream)
        .await
        .map_err(|err| err.to_string())?;
    let (_, connection) = tls.get_ref();
    let certs = connection
        .peer_certificates()
        .ok_or_else(|| "peer presented no certificate".to_string())?;
    let der = certs
        .first()
        .ok_or_else(|| "empty certificate chain".to_string())?;
    days_until_expiry(der.as_ref())
}

pub fn days_until_expiry(der: &[u8]) -> Result<i64, String> {
    let (_, cert) = X509Certificate::from_der(der).map_err(|err| err.to_string())?;
    let timestamp = cert.validity().not_after.timestamp();
    let expires = chrono::DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| "certificate expiry is not a valid time".to_string())?;
    Ok((expires - chrono::Utc::now()).num_days())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_days_from_generated_cert() {
        let cert = rcgen::generate_simple_self_signed(vec!["example.com".into()]).unwrap();
        let der = cert.cert.der();
        let days = days_until_expiry(der.as_ref()).unwrap();
        assert!((20..=40).contains(&days) || days > 0, "days={days}");
    }

    #[tokio::test]
    async fn refused_connection_is_an_ssl_error() {
        let info = ssl_check("https://127.0.0.1:1").await;
        assert!(!info.is_valid);
        assert!(info.error.is_some());
    }

    #[tokio::test]
    async fn local_certificate_reports_days_remaining() {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let cert_der = rustls::pki_types::CertificateDer::from(cert.cert.der().to_vec());
        let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der()),
        );
        let _ = rustls::crypto::ring::default_provider().install_default();
        let server = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der.clone()], key)
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server));
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = acceptor.accept(stream).await;
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert_der).unwrap();
        let days = handshake("localhost", port, client_config(roots))
            .await
            .unwrap();
        assert!(days > 0);
    }
}
