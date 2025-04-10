use tokio::sync::Mutex;
use std::{sync::Arc, io::{self, Write}};
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    pub player: Player,
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
    let connection = client.connect("127.0.0.1:8080".to_string()).await?;
    println!("Successfully connected to server!");

    tokio::spawn(QuicClient::listen_for_server_messages(connection.clone()));

    let game_state = Arc::new(Mutex::new(GameState {
        player: Player {
            x: 6,
            y: 4,
            hp: 100,
        },
    }));

    {
        let game_state = Arc::clone(&game_state);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                let state = game_state.lock().await;
                render_map(&state);
            }
        });
    }

    loop {
        let message = fetch_input().await;

        if let Some(msg) = message {
            match client.send_message(&connection, &msg).await {
                Ok(response) => {
                    // println!("Server response: {}", response)
                },
                Err(e) => eprintln!("Error sending message: {}", e),
            }
        }
    }

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


fn render_map(state: &GameState) {
    let map_width = 13;
    let map_height = 5;
    let mut map = vec![vec![' '; map_width]; map_height];

    std::process::Command::new("clear").status().unwrap();

    if state.player.y < map_height && state.player.x < map_width {
        map[state.player.y][state.player.x] = 'P';
    }

    println!("Map:");
    for row in map.iter() {
        println!("|{}|", row.iter().collect::<String>());
    }

    println!("\nPlayer Stats: {:?}", state.player);
}
