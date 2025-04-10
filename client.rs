use tokio::sync::Mutex;
use std::{io::{self, Write}, sync::{atomic::{AtomicBool, Ordering}, Arc}};
use quic_client::QuicClient;
mod quic_client;

use serde::{Serialize, Deserialize};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};


#[derive(Debug, Serialize, Deserialize)]
pub enum InputCommand {
    MoveLeft,
    MoveRight,
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    pub x: usize,
    pub y: usize,
    pub hp: u32,
    pub score : usize
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Enemy {
    pub x: usize,
    pub y: usize,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    pub player: Player,
    pub enemies : Vec<Enemy>,
    pub game_over: bool, 
    pub message : String
}

/*
Map Example
13x5 grid
 ___________
|           |
|  E        |
|     E F   |
|  F        |
|     ^     |
 -----------

forward is like 500ms
left right is like 100ms

*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = quic_client::QuicClient::new();

    println!("Connecting to QUIC server...");
    let connection = Arc::new(client.connect("127.0.0.1:8080".to_string()).await?);
    let clone_connection = Arc::clone(&connection);
    println!("Successfully connected to server!");
    
    // Create game state
    let game_state = Arc::new(Mutex::new(GameState {
        player: Player {
            x: 0,
            y: 0,
            hp: 100,
            score: 0
        },
        enemies: vec![],
        game_over: false,
        message: "".to_string()
    }));

    // Create shared input variable
    let latest_input = Arc::new(Mutex::new(String::from("None")));
    let latest_input_clone = Arc::clone(&latest_input);

    // Game running control flag
    let game_running = Arc::new(AtomicBool::new(true));
    let game_running_clone = Arc::clone(&game_running);
    
    // Spawn listener task
    {
        let connection_clone = Arc::clone(&connection);
        let latest_input_listener = Arc::clone(&latest_input);
        
        tokio::spawn(async move {
            loop {
                if !game_running_clone.load(Ordering::SeqCst) {
                    // Exit the loop if game is not running
                    break;
                }
                
                let backend_game_state = QuicClient::listen_for_server_messages(Arc::clone(&connection_clone)).await;
                
                if let Some(prompt) = render_map(&backend_game_state) {
                    if prompt == "prompt_restart" {
                        let mut user_input = String::new();
                        std::io::stdin().read_line(&mut user_input).expect("Failed to read input");
                        
                        match user_input.trim().to_lowercase().as_str() {
                            "r" => {
                                // Send restart command
                                let mut input_lock = latest_input_listener.lock().await;
                                *input_lock = "Restart".to_string();
                            },
                            "q" => {
                                // Signal to exit
                                game_running_clone.store(false, Ordering::SeqCst);
                                
                                // Send exit command to server
                                let mut input_lock = latest_input_listener.lock().await;
                                *input_lock = "Exit".to_string();
                                
                                break;
                            },
                            _ => {
                                println!("Invalid input. Enter 'r' to restart or 'q' to quit: ");
                            }
                        }
                    }
                }
            }
            
            println!("Game client shutting down...");
        });
    }

    // Main input loop
    while game_running.load(Ordering::SeqCst) {
        let input_cmd = fetch_input().await;

        if let Some(cmd) = input_cmd {
            // Update latest input
            {
                let mut input = latest_input_clone.lock().await;
                *input = cmd.clone();
            }
            
            // Send to server
            match client.send_message(&Arc::clone(&clone_connection), &cmd).await {
                Ok(_) => {
                    // Input sent successfully
                },
                Err(e) => eprintln!("Error sending message: {}", e),
            }
        } else {
            // Exit key pressed (ESC)
            game_running.store(false, Ordering::SeqCst);
            break;
        }
    }

    println!("Client shutting down...");
    Ok(())
}
async fn fetch_input() -> Option<String> {
    if let Err(_) = enable_raw_mode() {
        return None;
    }

    let result = if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
        if let Ok(Event::Key(key_event)) = event::read() {
            match key_event.code {
                KeyCode::Char('a') => Some("MoveLeft".to_string()),
                KeyCode::Char('d') => Some("MoveRight".to_string()),
                KeyCode::Char('r') => Some("Restart".to_string()),
                KeyCode::Char('q') => {
                    Some("Exit".to_string());
                    std::process::exit(0);
                },
                KeyCode::Left => Some("MoveLeft".to_string()),
                KeyCode::Right => Some("MoveRight".to_string()),
                KeyCode::Esc => None,
                _ => Some("None".to_string()),
            }
        } else {
            Some("None".to_string())
        }
    } else {
        Some("None".to_string())
    };

    let _ = disable_raw_mode();
    
    result
}


fn render_map(state: &GameState) -> Option<String> {
    let map_width = 13;
    let map_height = 5;
    let mut map = vec![vec![' '; map_width]; map_height];

    std::process::Command::new("clear").status().unwrap();
    
    if state.game_over {
        println!("{}", state.message);        
        return Some("prompt_restart".to_string());
    }

    if state.player.y < map_height && state.player.x < map_width {
        map[state.player.y][state.player.x] = 'P';
    }

    for enemy in &state.enemies {
        if enemy.y < map_height && enemy.x < map_width {
            map[enemy.y][enemy.x] = 'E';
        }
    }
    
    for row in map.iter().rev() {
        let row_string: String = row.iter().collect();
        print!(".{}.\n\r", row_string);
    }

    println!("\nPlayer Stats: {:?}", state.player);
    None
}



