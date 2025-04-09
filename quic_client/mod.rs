use std::{fs::File, io::BufReader, net::SocketAddr, sync::Arc};

use quinn::{ClientConfig, Connection, Endpoint};
use rustls::{RootCertStore};
use tokio::io::AsyncWriteExt;

pub struct QuicClient {
    pub endpoint: Endpoint,
}

impl QuicClient {
    pub fn new() -> Self {
        let endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap()).unwrap();
        Self {
            endpoint
        }
    }

    pub async fn connect(&mut self, server_addr: String) -> Result<Connection, Box<dyn std::error::Error>> {
        // Configure with root certificates
        let roots = generate_root_cert()?;
        let client_config = ClientConfig::with_root_certificates(Arc::new(roots));
        match client_config {
            Ok(config) => {
                self.endpoint.set_default_client_config(config);
            
                // Parse server address and connect
                let server_addr: SocketAddr = server_addr.parse()?;
                let connection = self.endpoint.connect(server_addr, "localhost")?.await?;
                println!("Connected to server: {}", connection.remote_address());
                
                Ok(connection)
            },
            Err(error) => Err(Box::new(error))
         }   
    }
    
    pub async fn send_message(&self, connection: &Connection, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Open a bidirectional stream
        let (mut send, mut recv) = connection.open_bi().await?;
        
        // Send the message
        send.write_all(message.as_bytes()).await?;
        send.finish();
        
        // Receive the response
        let mut buffer = vec![0u8; 1024];
        match recv.read(&mut buffer).await? {
            Some(bytes) => {
                let response = String::from_utf8_lossy(&buffer[..bytes]).to_string();
                Ok(response)
            },
            None => {
                Ok("No response received".to_string())
            }
        }
    }
}

fn generate_root_cert() -> Result<RootCertStore, Box<dyn std::error::Error>> {
    let cert_path = "cert.pem";
    let cert_file = File::open(cert_path)?;
    let mut reader = BufReader::new(cert_file);
    
    let mut roots: rustls::RootCertStore = rustls::RootCertStore::empty();

    // parse PEM certificates properly ( cna handle multiple)
    let certs = rustls_pemfile::certs(&mut reader);
    for cert in certs{
        match cert {
            Ok(certificates) => {
                roots.add(certificates)?;
            },
            Err(error) => println!("Error {:?}", error)
        }
    }
    
    Ok(roots)
}
