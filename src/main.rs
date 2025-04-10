use chrono::{DateTime, TimeZone, Utc};
use game::{GameState, InputCommand, Player};
use serde_json;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

mod quic_server;
use quic_server::QuicServer;
use tokio::sync::Mutex;

mod game;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let latest_input = Arc::new(Mutex::new(String::new()));

    let input_clone = Arc::clone(&latest_input);

    let custom_handler: Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync> =
        Arc::new(move |data: &[u8]| -> Vec<u8> {
            let input_clone = Arc::clone(&input_clone);
            let data_clone = data.to_vec();
            tokio::spawn(async move {
                if let Ok(message) = std::str::from_utf8(&data_clone) {
                    let mut input = input_clone.lock().await;
                    *input = message.to_string(); // Save to shared state
                    format!("Server processed: {}", message).into_bytes()
                } else {
                    let mut response = Vec::with_capacity(data_clone.len() + 1);
                    response.push(0x01);
                    response.extend_from_slice(&data_clone);
                    response
                }
            });

            Vec::new()
        });

    let server = Arc::new(QuicServer::new(
        "127.0.0.1:8080".to_string(),
        custom_handler.clone(),
    ));

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

    // {
    //     let server_clone = Arc::clone(&server);
    //     tokio::spawn(async move {
    //         loop {
    //             sleep(Duration::from_secs(5)).await;

    //             let msg = format!("Ping from server at {:?}", chrono::Utc::now());

    //             server_clone.broadcast(&msg.clone().into_bytes()).await;
    //             println!("Sent! {}", msg)
    //         }
    //     });
    // }

    // Game loop logic
    let state = Arc::new(Mutex::new(GameState {
        player: Player {
            x: 6,
            y: 4,
            hp: 100,
        },
    }));

    {
        let state = Arc::clone(&state);
        let inputs = Arc::clone(&latest_input);
        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(100)).await;
        
                let mut state = state.lock().await;
                let input = inputs.lock().await.clone();
        
                match input.as_str() {
                    "MoveLeft" => {
                        if state.player.x > 1 {
                            state.player.x -= 1;
                        }
                    }
                    "MoveRight" => {
                        if state.player.x < 11 {
                            state.player.x += 1;
                        }
                    }
                    "None" => {}
                    _ => {
                        println!("Unknown input: {}", input);
                    }
                }
        
                if state.player.y > 0 {
                    state.player.y -= 1;
                }
        
                let json = serde_json::to_string(&*state).unwrap();
                // println!("Broadcasting: {}", json); // Add logging for state
                server_clone.broadcast(json.as_bytes()).await;
            }
        });
        
    }
    let server_clone: Arc<QuicServer> = Arc::clone(&server);
    server_clone.accept_loop().await;

    Ok(())
}
