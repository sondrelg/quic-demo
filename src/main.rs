use std::{sync::Arc};

use color_eyre::eyre::{Result};
use quinn::Endpoint;

use message::Message;
use crate::client::send_client_message;
use crate::server::{configure_server, listen_server};

mod server;
mod message;
mod client;

extern crate pretty_env_logger;
#[macro_use] extern crate log;

pub const BUFFER_SIZE: usize = 1024;


#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    // Create Endpoint
    let server_address = "127.0.0.1:5454".parse().unwrap();
    let (server_config, server_cert) = configure_server()?;
    let endpoint = Endpoint::server(server_config, server_address)?;

    // Spawn server
    tokio::spawn(listen_server(Arc::new(endpoint)));

    // Send 10 hello messages from different connections
    let tasks: Vec<_> = (0..10)
        .map(|_| tokio::spawn(send_client_message(Message::Hello, server_cert.clone(), server_address)))
        .collect();

    // Run tasks in parallel
    for task in tasks {
        task.await??;
    }

    // Send shutdown message
    send_client_message(Message::Shutdown, server_cert.clone(), server_address).await?;
    Ok(())
}
