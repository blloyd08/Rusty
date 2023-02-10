use std::{collections::HashMap, sync::Arc};

use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

use crate::{
    game_task::{GameCommand, GameTask},
    types::Direction,
    GameError, GameState, JoinGameReply,
};
pub(crate) struct GameManager {
    games: Arc<Mutex<HashMap<String, GameTask>>>,
}

impl Default for GameManager {
    fn default() -> Self {
        Self {
            games: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl GameManager {
    pub(crate) fn new() -> Self {
        Self {
            games: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) async fn create_game(
        &self,
        width: i32,
        height: i32,
        tick_duration_millis: u64,
    ) -> String {
        let game = GameTask::new(width, height, tick_duration_millis);
        let game_id = Uuid::new_v4().to_string();
        println!("Creating game {}", game_id);
        let mut games = self.games.lock().await;
        games.insert(game_id.clone(), game);
        game_id
    }

    pub(crate) async fn join_game(&self, game_id: String) -> Result<JoinGameReply, GameError> {
        match self.games.lock().await.get(&game_id) {
            None => Err(GameError::InvalidGame),
            Some(game) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                let cmd = GameCommand::JoinGame {
                    reply_sender: resp_tx,
                };
                game.send_command(cmd).await;

                // Await the response
                match resp_rx.await {
                    Ok(reply) => {
                        println!("User {} joined game {}", reply.user_id, game_id);
                        Ok(reply)
                    }
                    Err(err) => {
                        print!("JoinGameReply internal error: {}", err);
                        Err(GameError::Internal)
                    }
                }
            }
        }
    }

    pub(crate) async fn start_game(
        &self,
        game_id: String,
        user_id: String,
    ) -> Result<(), GameError> {
        match self.games.lock().await.get(&game_id) {
            None => Err(GameError::InvalidGame),
            Some(game) => {
                println!("User {} is starting game {}", user_id, game_id);
                let (resp_tx, resp_rx) = oneshot::channel();
                let cmd = GameCommand::StartGame {
                    reply_sender: resp_tx,
                    user_id,
                };
                game.send_command(cmd).await;

                // Await the response
                match resp_rx.await {
                    Ok(game_error_opt) => match game_error_opt {
                        Some(game_error) => Err(game_error),
                        None => Ok(()),
                    },
                    Err(err) => {
                        println!("Start game received error: {}", err);
                        Err(GameError::Internal)
                    }
                }
            }
        }
    }

    pub(crate) async fn update_game(
        &self,
        game_id: String,
        user_id: String,
        direction: Direction,
    ) -> Result<GameState, GameError> {
        let games = self.games.lock().await;
        match games.get(&game_id) {
            None => Err(GameError::InvalidGame),
            Some(game) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                let cmd = GameCommand::UpdateGame {
                    reply_sender: resp_tx,
                    user_id,
                    direction,
                };
                game.send_command(cmd).await;

                // Await the response
                match resp_rx.await {
                    Ok(result) => result,
                    Err(err) => {
                        println!("Internal error receiving update game response: {}", err);
                        Err(GameError::Internal)
                    }
                }
            }
        }
    }

    pub(crate) async fn game_status(
        &self,
        game_id: String,
        user_id: String,
    ) -> Result<GameState, GameError> {
        let games = self.games.lock().await;
        match games.get(&game_id) {
            None => Err(GameError::InvalidGame),
            Some(game) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                let cmd = GameCommand::GameStatus {
                    reply_sender: resp_tx,
                    user_id,
                };
                game.send_command(cmd).await;

                // Await the response
                match resp_rx.await {
                    Ok(result) => result,
                    Err(err) => {
                        println!("Internal error getting GameState: {}", err);
                        Err(GameError::Internal)
                    }
                }
            }
        }
    }
}
