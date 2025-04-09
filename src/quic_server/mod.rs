use std::collections::HashMap;
use std::{fs::File, sync::Arc};
use std::io::Write;
use std::net::SocketAddr;

use quinn::{ Connection, Endpoint, Incoming, RecvStream, SendStream, ServerConfig};
use rcgen::{generate_simple_self_signed, CertifiedKey};
use rustls::lock::Mutex;
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};


pub type MessageHandler = Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>;

/*
QuicServer
- generate certificate
- make server config
- create and bind endpoint
- accept connection
- handle message from stream
*/
pub struct QuicServer {
    // QUIC needs the Endpoint to stay alive while the server is running. 
    // If it's dropped, the server stops listening.
    // By storing it inside the struct, you:
    // - Tie its lifetime to your server
    // - Prevent accidental early drops
    endpoint: Endpoint,
    message_handler: MessageHandler,

    // for storing multiple client so i can send message indvidually 
    pub connections: Arc<Mutex<HashMap<SocketAddr, Connection>>>
}

impl QuicServer {    
    // creates server
    pub fn new(endpoint_addr: String, message_handler: MessageHandler) -> Self {
        let addr: SocketAddr = endpoint_addr.parse().unwrap();
        let server_config = generate_server_config();
        let endpoint = Endpoint::server(server_config, addr)
            .expect("Failed to create endpoint");

        Self {
            endpoint,
            message_handler,
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // handle connection 
    pub async fn accept_loop(&self) {
        println!("Server listening on {}", self.endpoint.local_addr().unwrap());
        
        while let Some(connecting) = self.endpoint.accept().await {
            let handler = Arc::clone(&self.message_handler);
            tokio::spawn(async move {
                handle_connection(connecting, handler).await;
            });
        }
    }
    

    // to send message back to client
    pub async fn broadcast(&self, message: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let connections = self.connections.lock().unwrap();
        
        for connection in connections.values() {
            println!("{:?}", connection);
            self.send_message_to_client(connection, message).await?;
        }
        
        Ok(())
    }
    
    pub async fn send_message_to_client(
        &self, 
        connection: &Connection, 
        message: &[u8]
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut send, _recv) = connection.open_bi().await?;
        
        send.write_all(message).await?;
        send.finish();
        
        Ok(())
    }

    pub fn list_connections(&self) -> Arc<Mutex<HashMap<SocketAddr, Connection>>> {
        self.connections.clone()
    }
    
    fn set_connection(&self){
        // self.connections.lock().set
    }
}



// helper for handle connections
async fn handle_connection(connecting: Incoming, message_handler: MessageHandler) {
    match connecting.await {
        Ok(connection) => {
            println!("Connection established from: {}", connection.remote_address());
            if let Err(e) = process_connection(connection, message_handler).await {
                eprintln!("Connection error: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Connection failed: {}", e);
        }
    }
}

async fn process_connection(
    connection: Connection,
    message_handler: MessageHandler
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match connection.accept_bi().await {
            Ok((send, recv)) => {
                if let Err(e) = handle_stream(send, recv, Arc::clone(&message_handler)).await {
                    eprintln!("Stream error: {}", e);
                }
            },
            Err(e) => {
                println!("Connection ended: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

async fn handle_stream(
    mut send: SendStream,
    mut recv: RecvStream,
    message_handler: MessageHandler
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0; 1024];
    
    match recv.read(&mut buffer).await? {
        Some(bytes) => {
            let data = &buffer[..bytes];
            
            if let Ok(message) = std::str::from_utf8(data) {
                println!("Received message: {}", message);
            } else {
                println!("Received binary data: {} bytes", bytes);
            }
            
            let response = message_handler(data);
            
            send.write_all(&response).await?;
            send.finish();
        },
        None => {
            println!("Empty stream received");
        }
    }
    
    Ok(())
}



// handling tls
pub fn generate_sign_cert() -> (String, String) {
    let subject_alt_names = vec!["localhost".to_string()];
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names).unwrap();
    (cert.pem(), key_pair.serialize_pem())
}

pub fn generate_server_config() -> ServerConfig {
    let (cert_pem, key_pem) = generate_sign_cert();

    let cert_path = "cert.pem";
    let key_path = "key.pem";

    File::create(cert_path)
        .unwrap()
        .write_all(cert_pem.as_bytes())
        .unwrap();

    File::create(key_path)
        .unwrap()
        .write_all(key_pem.as_bytes())
        .unwrap();

    let certs: Vec<CertificateDer> = CertificateDer::pem_file_iter(cert_path)
        .unwrap()
        .map(|cert| cert.unwrap())
        .collect();

    let private_key = PrivateKeyDer::from_pem_file(key_path).unwrap();

    let mut server_config: quinn::ServerConfig =
        quinn::ServerConfig::with_single_cert(certs, private_key).unwrap();
    server_config
}
