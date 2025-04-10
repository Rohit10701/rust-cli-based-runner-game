use chrono::{DateTime, Local, TimeZone, Utc};
use game::{Enemy, GameState, InputCommand, Player};
use serde_json;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use tokio::time::{sleep, Duration};

mod quic_server;
use quic_server::QuicServer;
use tokio::sync::Mutex;
mod game;
use rand::Rng;
extern crate rand;





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

    let player = Player {
        x: 5,
        y: 1,
        hp: 100,
        score: 0,
    };

    let enemies = vec![Enemy { x: 1, y: 9 }, Enemy { x: 3, y: 5 }];
    let game_over = false;
    let message = "".to_string();

    let state = Arc::new(Mutex::new(GameState { player, enemies, game_over, message }));

    {
        let state = Arc::clone(&state);
        let inputs = Arc::clone(&latest_input);
        let server_clone = Arc::clone(&server);
        let game_running = Arc::new(AtomicBool::new(true));
        let game_running_clone = Arc::clone(&game_running);
        
        let mut score_timer: DateTime<Utc> = Utc::now();
        let mut enemy_timer: DateTime<Utc> = Utc::now();
        let tick_duration = Duration::from_millis(1000 / 60); // 60Hz -> 16.67ms per tick
        
        tokio::spawn(async move {
            while game_running_clone.load(Ordering::SeqCst) {
                let tick_start = tokio::time::Instant::now(); 
                let mut state = state.lock().await;
                let input = inputs.lock().await.clone();
                
                if input.as_str() == "Exit" {
                    state.message = "Game shutting down...".to_string();
                    game_running_clone.store(false, Ordering::SeqCst);
                    
                    let json = serde_json::to_string(&*state).unwrap();
                    server_clone.broadcast(json.as_bytes()).await;
                    break;
                }
            
                
                if !state.game_over {
                    let current_time: DateTime<Utc> = Utc::now();
                    
                    if (current_time - score_timer).num_milliseconds() > 1000 {
                        state.player.score += 1;
                        score_timer = current_time;
                    }
            
                    if (current_time - enemy_timer).num_milliseconds() > 500 {
                        let random_x = rand::thread_rng().gen_range(1, 11);
            
                        for enemy in state.enemies.iter_mut() {
                            enemy.y -= 1;
                        }
            
                        state.enemies.retain(|enemy| enemy.y > 0);
            
                        while state.enemies.len() < 3 {
                            state.enemies.insert(0, Enemy { x: random_x, y: 11 });
                        }
                        
                        enemy_timer = current_time;
                    }
            
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
                        "Restart" | "Exit" => {
                            println!("Unknown input: {}", input);
                            state.player = Player { x: 5, y: 1, score: 0, hp:100 };
                            state.enemies =  vec![Enemy { x: 1, y: 9 }, Enemy { x: 3, y: 5 }];
                            state.game_over = false;
                            state.message = "".to_string();
                            score_timer = Utc::now();
                            enemy_timer = Utc::now();
                        }
                        _ => {
                            println!("Unknown input: {}", input);
                        }
                    }
                    
                    for enemy in &state.enemies {

                        println!("crashed {:?} - {:?}", enemy, state.player);
                        if enemy.x == state.player.x && enemy.y == state.player.y {
                            state.game_over = true;
                            state.message = "Game Over! Press 'r' to restart or 'q' to quit".to_string();

                            break;
                        }
                    }
                }
        
                // println!("{:?}", *state);
                let json = serde_json::to_string(&*state).unwrap();
        
                let processing_time = tick_start.elapsed();
                let remaining_time = tick_duration.saturating_sub(processing_time);
        
                sleep(remaining_time).await;
                server_clone.broadcast(json.as_bytes()).await;
            }
            
            println!("Game server loop terminated.");
        });
    }


    let server_clone: Arc<QuicServer> = Arc::clone(&server);
    server_clone.accept_loop().await;

    Ok(())
}
