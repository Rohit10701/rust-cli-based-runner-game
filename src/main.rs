use std::fs::File;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use quinn::{Connecting, Connection, Endpoint, Incoming};
use rcgen::{generate_simple_self_signed, CertifiedKey};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ServerConfig, ServerConnection};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use quinn::ReadExactError;

mod common;


// game
mod game;


#[tokio::main]
async fn main() {
    setup_tls().await;
}


async fn setup_tls() {
    let (cert_pem, key_pem) = sign_cert_for_quic();

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

    let mut server_config = quinn::ServerConfig::with_single_cert(certs, private_key).unwrap();

    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut endpoint = Endpoint::server(server_config, addr).expect("Failed to create endpoint");

    println!("QUIC server listening on {}", addr);

    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(handle_connection(connecting));
    }


}

async fn handle_connection(connecting: Incoming) {
    match connecting.await {
        Ok(connection) => {
            println!("Connection established from: {}", connection.remote_address());
            
            while let Ok((mut send, mut recv)) = connection.accept_bi().await {
                tokio::spawn(async move {
                    let mut buffer = vec![0; 1024];
                    
                    match recv.read(&mut buffer).await {
                        Ok(n) => {
                            match n {
                                Some(n) =>{ 
                                    if n > 0 {
                                    match String::from_utf8(buffer[..n].to_vec()) {
                                        Ok(message) => {
                                            println!("Received message: {}", message);
                                            
                                            let response = format!("Server received: {}", message);
                                            if let Err(e) = send.write_all(response.as_bytes()).await {
                                                eprintln!("Failed to send response: {}", e);
                                            }
                                        }
                                        Err(e) => eprintln!("Invalid UTF-8: {}", e),
                                    }
                                }},
                                None => println!("No data found")
                            }
                        }
                        Ok(_) => println!("Connection closed"),
                        Err(e) => eprintln!("Error reading from stream: {}", e),
                    }
                });
            }
        }
        Err(err) => {
            eprintln!("Failed to accept connection: {}", err);
        }
    }
}

fn sign_cert_for_quic() -> (String, String) {
    let subject_alt_names = vec!["localhost".to_string()];

    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names).unwrap();

    (cert.pem(), key_pair.serialize_pem())
}
