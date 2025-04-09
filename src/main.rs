use std::sync::Arc;
mod quic_server;
use quic_server::QuicServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let custom_handler = Arc::new(|data: &[u8]| -> Vec<u8> {
        if let Ok(message) = std::str::from_utf8(data) {
            format!("Server processed: {}", message).into_bytes()
        } else {
            let mut response = Vec::with_capacity(data.len() + 1);
            response.push(0x01);
            response.extend_from_slice(data);
            response
        }
    });

    let server = Arc::new(QuicServer::new("127.0.0.1:8080".to_string(), custom_handler));
    
    server.accept_loop().await;
    Ok(())
}