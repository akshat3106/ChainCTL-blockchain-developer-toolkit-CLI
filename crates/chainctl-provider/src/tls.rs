use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::net::TcpStream;

/// Connects to `host:443` and reads the leaf certificate's expiry date.
///
/// This is deliberately a *second*, separate TLS handshake from the actual
/// HTTP health check in `health::check_one` — reqwest doesn't expose the
/// peer certificate through its public API, so this does its own raw
/// handshake purely to read that field. Certificate *trust* was already
/// established by the real HTTPS request succeeding; this one uses the same
/// OS-native TLS stack (`native-tls`) with its normal validation, so a
/// failure here (unreachable, handshake error, unparseable cert) just means
/// "expiry unknown," not "insecure" — callers get `None` and move on rather
/// than treating it as a hard error (ARCHITECTURE.md §9/§15).
pub async fn certificate_expiry(host: &str, timeout: Duration) -> Option<DateTime<Utc>> {
    tokio::time::timeout(timeout, fetch_expiry(host))
        .await
        .ok()
        .flatten()
}

async fn fetch_expiry(host: &str) -> Option<DateTime<Utc>> {
    let tcp = TcpStream::connect((host, 443)).await.ok()?;
    let connector: tokio_native_tls::TlsConnector = native_tls::TlsConnector::new().ok()?.into();
    let tls_stream = connector.connect(host, tcp).await.ok()?;
    let cert = tls_stream.get_ref().peer_certificate().ok()??;
    let der = cert.to_der().ok()?;
    parse_not_after(&der)
}

fn parse_not_after(der: &[u8]) -> Option<DateTime<Utc>> {
    let (_, parsed) = x509_parser::parse_x509_certificate(der).ok()?;
    DateTime::from_timestamp(parsed.validity().not_after.timestamp(), 0)
}
