use std::io;

use quic_client::QuicClient;
mod quic_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = quic_client::QuicClient::new();
    
    println!("Connecting to QUIC server...");
    let connection = client.connect("127.0.0.1:8080".to_string()).await?;
    println!("Successfully connected to server!");
    tokio::spawn(QuicClient::listen_for_server_messages(connection.clone()));

    loop {
        let mut input = String::new();
        println!("Enter a message (or empty line to quit): ");
        io::stdin().read_line(&mut input)?;
        
        let message = input.trim();
        if message.is_empty() {
            println!("Exiting...");
            break;
        }
        
        match client.send_message(&connection, message).await {
            Ok(response) => println!("Server response: {}", response),
            Err(e) => eprintln!("Error sending message: {}", e)
        }
    }
    
    Ok(())
}