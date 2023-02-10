use game_manager::GameManager;
use tokio::sync::oneshot;
use types::Direction;

mod game;
mod game_manager;
mod game_task;
pub mod output;
mod requested_direction;
pub mod service;
mod types;

pub mod proto {
    tonic::include_proto!("rusty");
}

pub struct RustyGame {
    manager: GameManager,
}

impl Default for RustyGame {
    fn default() -> Self {
        Self {
            manager: GameManager::new(),
        }
    }
}

impl RustyGame {
    pub fn new() -> Self {
        RustyGame {
            manager: GameManager::new(),
        }
    }

    pub async fn create_game(&self, width: i32, height: i32, tick_duration_millis: u64) -> String {
        self.manager
            .create_game(width, height, tick_duration_millis)
            .await
    }

    pub async fn join_game(&self, game_id: String) -> Result<JoinGameReply, GameError> {
        self.manager.join_game(game_id).await
    }

    pub async fn start_game(&self, game_id: String, user_id: String) -> Result<(), GameError> {
        self.manager.start_game(game_id, user_id).await
    }

    pub async fn game_status(
        &self,
        game_id: String,
        user_id: String,
    ) -> Result<GameState, GameError> {
        self.manager.game_status(game_id, user_id).await
    }

    pub async fn update_game(
        &self,
        game_id: String,
        user_id: String,
        direction: Direction,
    ) -> Result<GameState, GameError> {
        self.manager.update_game(game_id, user_id, direction).await
    }
}

#[derive(Debug)]
pub struct JoinGameReply {
    pub user_id: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug)]
pub enum GameError {
    InvalidUser,
    InvalidGame,
    Internal,
}

impl From<Direction> for proto::MoveDirection {
    fn from(s: Direction) -> Self {
        match s {
            Direction::North => proto::MoveDirection::North,
            Direction::East => proto::MoveDirection::East,
            Direction::South => proto::MoveDirection::South,
            Direction::West => proto::MoveDirection::West,
        }
    }
}

impl From<i32> for Direction {
    fn from(s: i32) -> Self {
        match proto::MoveDirection::from_i32(s).unwrap() {
            proto::MoveDirection::East => Direction::East,
            proto::MoveDirection::North => Direction::North,
            proto::MoveDirection::South => Direction::South,
            proto::MoveDirection::West => Direction::West,
        }
    }
}

impl From<GameState> for proto::GameState {
    fn from(game_state: GameState) -> Self {
        Self {
            number_of_players: game_state.num_users,
            food: Some(game_state.food.into()),
            body: game_state.body.into_iter().map(|p| p.into()).collect(),
            move_direction: proto::MoveDirection::into(game_state.direction.into()),
        }
    }
}

impl From<Point> for proto::Point {
    fn from(point: Point) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn add_direction(&self, direction: &Direction) -> Point {
        match direction {
            Direction::North => Point::new(self.x, self.y - 1),
            Direction::South => Point::new(self.x, self.y + 1),
            Direction::East => Point::new(self.x + 1, self.y),
            Direction::West => Point::new(self.x - 1, self.y),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GameState {
    pub height: i32,
    pub width: i32,
    pub tick: usize,
    pub game_over_reason: Option<GameOverReason>,
    pub direction: Direction,
    pub num_users: u32,
    pub body: Vec<Point>,
    pub food: Point,
}

/// Provided by the requester and used by the manager task to send
/// the command response back to the requester.
type Responder<T> = oneshot::Sender<T>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameOverReason {
    OutOfBounds,
    CollideWithSelf,
    // Rusty has filled every available space
    Winner,
}

#[cfg(test)]
mod tests {
    use crate::{types::Direction, Point};

    #[test]
    fn add_direction() {
        let test_values: [(Direction, Point); 4] = [
            (Direction::North, Point::new(0, -1)),
            (Direction::South, Point::new(0, 1)),
            (Direction::East, Point::new(1, 0)),
            (Direction::West, Point::new(-1, 0)),
        ];

        for test_value in test_values {
            let (direction, expected_point) = test_value;
            assert_eq!(Point::new(0, 0).add_direction(&direction), expected_point);
        }
    }
}
