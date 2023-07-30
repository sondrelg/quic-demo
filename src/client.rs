use std::net::SocketAddr;
use std::time::{Duration, Instant};

use color_eyre::eyre::{eyre, Result};
use quinn::{ClientConfig, Endpoint};
use crate::BUFFER_SIZE;

use crate::message::Message;

/// Build a default quinn client config which trusts given certificates.
/// param server_certs: a list of trusted certificates in DER format.
fn configure_client(server_certs: &[&[u8]]) -> Result<ClientConfig> {
    let mut certs = rustls::RootCertStore::empty();
    for cert in server_certs {
        certs.add(&rustls::Certificate(cert.to_vec()))?;
    }
    let client_config = ClientConfig::with_root_certificates(certs);
    Ok(client_config)
}

fn duration_secs(x: &Duration) -> f32 {
    x.as_secs() as f32 + x.subsec_nanos() as f32 * 1e-9
}

pub async fn send_client_message(message: Message, server_cert: Vec<u8>, server_addr: SocketAddr) -> Result<()> {
    // Create Endpoint
    let client_cfg = configure_client(&[&server_cert])?;
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())?;
    endpoint.set_default_client_config(client_cfg);

    // Connect to server
    let conn = endpoint.connect(server_addr, "localhost")?.await?;
    debug!("connected: addr={}", conn.remote_address());

    // Transmit a message
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| eyre!("failed to open stream: {}", e))?;
    send.write_all(bytes::Bytes::from(message).as_ref()).await.map_err(|e| eyre!("failed to send request: {}", e))?;
    send.finish().await.map_err(|e| eyre!("failed to shutdown stream: {}", e))?;

    // Read response
    let response_start = Instant::now();
    let response = recv.read_to_end(BUFFER_SIZE).await.map_err(|e| eyre!("failed to read response: {}", e))?;
    let duration = response_start.elapsed();
    debug!(
        "response received in {:?} - {} KiB/s",
        duration,
        response.len() as f32 / (duration_secs(&duration) * 1024.0)
    );

    let response_message = Message::from(response);
    info!("received response: message={:?}", response_message);

    conn.close(0u32.into(), b"done");

    // Give the server a fair chance to receive the close packet
    endpoint.wait_idle().await;

    Ok(())
}