use std::sync::Arc;
use chrono::{DateTime, TimeZone, Utc};
use rustls::lock::Mutex;
use tokio::time::{sleep, Duration};

mod quic_server;
use quic_server::QuicServer;

mod game;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let latest_input = Arc::new(Mutex::new(String::new()));

    let input_clone = Arc::clone(&latest_input);

    let custom_handler = Arc::new(move |data: &[u8]| -> Vec<u8> {
        if let Ok(message) = std::str::from_utf8(data) {
            let mut input = input_clone.lock().unwrap();
            *input = message.to_string(); // Save to shared state
            format!("Server processed: {}", message).into_bytes()
        } else {
            let mut response = Vec::with_capacity(data.len() + 1);
            response.push(0x01);
            response.extend_from_slice(data);
            response
        }
    });

    let server = Arc::new(QuicServer::new(
        "127.0.0.1:8080".to_string(),
        Arc::new(custom_handler.clone()),
    ));

    // checks for all the client connection
    {
        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                let map = server_clone.connections.lock().unwrap();
                println!("Client connections: {:?}", map.keys());
            }
        });
    }

    // Ping from server
    {
        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;

                let msg = format!("Ping from server at {:?}", chrono::Utc::now());

                server_clone.broadcast(&msg.clone().into_bytes()).await;
                println!("Sent! {}", msg)
            }
        });
    }

    // game loop logic
    // https://gameprogrammingpatterns.com/game-loop.html
    {
        let server_clone = Arc::clone(&server);
        let player_inputs = custom_handler.clone();
        tokio::spawn(async move{
            let mut last_time: DateTime<Utc> = Utc.with_ymd_and_hms(2015, 5, 15, 0, 0, 0).unwrap();
            loop {
                let current_time: DateTime<Utc> = Utc.with_ymd_and_hms(2015, 5, 15, 0, 0, 0).unwrap();
                let elapsed_time = current_time - last_time;
                // processInput();
                // update(elapsed);
                // render();
                last_time = current_time;
            }
        });
    }
    server.accept_loop().await;

    Ok(())
}
