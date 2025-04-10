use serde::{Serialize, Deserialize};

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
}

/*
Map Example
13x5 gird
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


