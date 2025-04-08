use std::time::{Duration, Instant};
use std::thread;
use std::io::{self, Write};

enum Tiles {
    Wall,
    Floor,
    Fruit,
    Enemy,
    Player
}

pub struct Player {
    pub x : usize,
    pub y : usize,
    pub hp: u32
}

enum Direction {
    Left,
    Right
}

pub fn main(direction : Direction){
    let mut player = Player { x: 0, y: 0, hp : 100 };
    let tick_rate = Duration::from_millis(100);

}