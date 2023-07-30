use std::{ascii, sync::Arc};
use std::process::exit;

use color_eyre::eyre::{eyre, Result};
use quinn::{Endpoint, ServerConfig};
use crate::BUFFER_SIZE;
use crate::message::Message;


/// Returns default server configuration along with its certificate.
pub fn configure_server() -> Result<(ServerConfig, Vec<u8>)> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_der = cert.serialize_der().unwrap();
    let priv_key = cert.serialize_private_key_der();
    let priv_key = rustls::PrivateKey(priv_key);
    let cert_chain = vec![rustls::Certificate(cert_der.clone())];
    let mut server_config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(5_u8.into());
    Ok((server_config, cert_der))
}

pub async fn listen_server(endpoint: Arc<Endpoint>) -> Result<()> {
    while let Some(conn) = endpoint.accept().await {
        let fut = handle_connection(conn);
        tokio::spawn(async move {
            if let Err(e) = fut.await {
                error!("connection failed: {e}")
            }
        });
    }

    // Dropping all handles associated with a connection implicitly closes it
    Ok(())
}


/// Each stream initiated by the client constitutes a new request.
async fn handle_connection(conn: quinn::Connecting) -> Result<()> {
    let connection = conn.await?;
    async {
        debug!("received connection: addr={}", connection.remote_address());
        loop {
            let stream = connection.accept_bi().await;
            let stream = match stream {
                Ok(s) => s,
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    debug!("connection closed");
                    return Ok(());
                }
                Err(e) => {
                    error!("error: {e}");
                    return Err(e);
                }
            };
            tokio::spawn(handle_request(stream));
        }
    }
        .await?;
    Ok(())
}


async fn handle_request((mut send, mut recv): (quinn::SendStream, quinn::RecvStream)) -> Result<()> {
    // Read request content
    let request = recv.read_to_end(BUFFER_SIZE).await?;
    let content = request
        .iter()
        .flat_map(|&x| ascii::escape_default(x))
        .map(|x| x as char)
        .collect::<String>();

    // Cast to message
    let message = Message::from(content);
    info!("received request: message={:?}", message);

    // Write back a response
    send.write_all((&message).into())
        .await
        .map_err(|e| eyre!("failed to send response: {}", e))?;

    // Gracefully terminate the stream
    send.finish()
        .await
        .map_err(|e| eyre!("failed to shutdown stream: {}", e))?;

    // Shut down if told to
    if let Message::Shutdown = message {
        debug!("shutting down");
        exit(0)
    }

    Ok(())
}
