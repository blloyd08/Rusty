use log::{info, warn};
use std::time::Duration;
use std::{sync::Arc, sync::Weak};

use tokio::{
    sync::mpsc::{self, Sender},
    task::JoinHandle,
    time::{self},
};
use uuid::Uuid;

use crate::{game::Game, types::Direction, GameState, Responder};
use crate::{GameError, JoinGameReply};

pub(crate) struct GameTask {
    _manager: JoinHandle<()>,
    sender: Arc<Sender<GameCommand>>,
}

impl GameTask {
    pub fn new(width: i32, height: i32, tick_duration_millis: u64) -> Self {
        let (tx, mut rx) = mpsc::channel::<GameCommand>(32);
        let sender = Arc::new(tx);
        let weak_game_sender = Arc::downgrade(&sender);

        // The `move` keyword is used to **move** ownership of `rx` into the task.
        let _manager = tokio::spawn(async move {
            let game_sender = weak_game_sender;
            let max_spaces: usize = (width * height).try_into().unwrap();
            let mut game = Game::new(height, width);
            let mut _tick_handle = None;
            // Start receiving messages
            while let Some(cmd) = rx.recv().await {
                use GameCommand::*;

                match cmd {
                    GameStatus {
                        reply_sender,
                        user_id,
                    } => {
                        GameTask::game_status(reply_sender, user_id, &mut game).await;
                    }
                    UpdateGame {
                        reply_sender,
                        user_id,
                        direction,
                    } => {
                        GameTask::update_game(reply_sender, user_id, direction, &mut game).await;
                    }
                    JoinGame { reply_sender } => {
                        GameTask::join_game(reply_sender, &mut game).await;
                    }
                    StartGame {
                        reply_sender,
                        user_id,
                    } => {
                        let reply = match GameTask::start_game(
                            user_id,
                            &mut game,
                            tick_duration_millis,
                            game_sender.clone(),
                        )
                        .await
                        {
                            Ok(tick_handle) => {
                                _tick_handle = Some(tick_handle);
                                None
                            }
                            Err(err) => Some(err),
                        };
                        reply_sender
                            .send(reply)
                            .expect("Start Game response should succeed");
                    }
                    Tick {} => {
                        let game_state = GameTask::tick(&mut game, max_spaces).await;
                        let game_over = game_state.game_over_reason.is_some();
                        if game_over {
                            break;
                        }
                    }
                }
            }
            warn!("Exiting game loop");
        });

        Self { _manager, sender }
    }

    pub async fn send_command(&self, command: GameCommand) {
        if let Err(error) = self.sender.send(command).await {
            print!("Send game command failed due to error: {}", error);
        }
    }

    async fn game_status(
        reply_sender: Responder<Result<GameState, GameError>>,
        user_id: String,
        game: &mut Game,
    ) {
        if game.user_has_joined_game(user_id).await {
            let _ = reply_sender.send(Ok(game.into_game_state().await));
        } else {
            let _ = reply_sender.send(Err(GameError::InvalidUser));
        }
    }

    async fn update_game(
        reply_sender: Responder<Result<GameState, GameError>>,
        user_id: String,
        direction: Direction,
        game: &mut Game,
    ) {
        if !game.user_has_joined_game(user_id.clone()).await {
            let _ = reply_sender.send(Err(GameError::InvalidUser));
            return;
        }
        game.add_user_direction(user_id, direction).await;

        let game_state = game.into_game_state().await;
        let _ = reply_sender.send(Ok(game_state));
    }

    async fn join_game(join_game_reply_receiver: Responder<JoinGameReply>, game: &mut Game) {
        let user_id = Uuid::new_v4().to_string();
        let _user_is_added = game.add_user(user_id.clone()).await;
        let (width, height) = game.get_dimensions();

        // Ignore errors
        let _ = join_game_reply_receiver.send(JoinGameReply {
            user_id,
            width: width as i32,
            height: height as i32,
        });
    }

    async fn start_game(
        user_id: String,
        game: &mut Game,
        tick_duration_millis: u64,
        command_sender: Weak<Sender<GameCommand>>,
    ) -> Result<JoinHandle<()>, GameError> {
        if game.user_has_joined_game(user_id).await {
            let _tick = tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_millis(tick_duration_millis));
                // Sleep On initial start to allow users time to react after starting game
                tokio::time::sleep(Duration::from_secs(3)).await;
                loop {
                    interval.tick().await;
                    if let Some(tick_sender) = command_sender.upgrade() {
                        match tick_sender.send(GameCommand::Tick {}).await {
                            Ok(_) => info!("Tick!"),
                            Err(_) => {
                                warn!("Failed to send tick. Channel Closed");
                                break;
                            }
                        }
                    } else {
                        warn!("Command sender dropped. Exiting tick loop");
                        break;
                    }
                }
            });
            return Ok(_tick);
        }
        Err(GameError::InvalidUser)
    }

    async fn tick(game: &mut Game, max_spaces: usize) -> GameState {
        game.tick(max_spaces).await;
        game.into_game_state().await
    }
}

pub(crate) enum GameCommand {
    UpdateGame {
        reply_sender: Responder<Result<GameState, GameError>>,
        user_id: String,
        direction: Direction,
    },
    GameStatus {
        reply_sender: Responder<Result<GameState, GameError>>,
        user_id: String,
    },
    JoinGame {
        reply_sender: Responder<JoinGameReply>,
    },
    StartGame {
        reply_sender: Responder<Option<GameError>>,
        user_id: String,
    },
    Tick {},
}

#[cfg(test)]
mod tests {
    use crate::game_task::GameState;
    use crate::output::print_world;
    use crate::Point;
    use tokio::sync::oneshot::{self};

    use crate::{
        game_task::{GameCommand, GameTask},
        types::Direction,
    };

    #[tokio::test]
    async fn create_game_command() {
        get_test_game();
    }

    #[tokio::test]
    async fn join_game_command() {
        let game_task = get_test_game();
        join_game(&game_task).await;
    }

    #[tokio::test]
    async fn update_game_command() {
        let game_task = get_test_game();
        let user_id = join_game(&game_task).await;

        let (resp, resp_rx) = oneshot::channel();
        // Send the create game request
        let cmd = GameCommand::UpdateGame {
            reply_sender: resp,
            user_id,
            direction: Direction::South,
        };

        game_task.send_command(cmd).await;

        // Await the response
        let res = resp_rx.await;
        let game_state = res.unwrap().unwrap();
        const HEIGHT: i32 = 10;
        let expected_game_state = GameState {
            tick: 0,
            game_over_reason: None,
            direction: Direction::South,
            num_users: 1,
            body: vec![
                Point::new(2, HEIGHT / 2),
                Point::new(1, HEIGHT / 2),
                Point::new(0, HEIGHT / 2),
            ],
            height: HEIGHT,
            width: HEIGHT,
            food: Point::new(0, 0),
        };
        println!("Actual:");
        print_world(&game_state);
        println!("Expected:");
        print_world(&expected_game_state);
        assert_eq!(game_state, expected_game_state);
    }

    fn get_test_game() -> GameTask {
        GameTask::new(10, 10, 1000)
    }

    async fn join_game(game_task: &GameTask) -> String {
        let (resp, resp_rx) = oneshot::channel();
        // Send the create game request
        let cmd = GameCommand::JoinGame { reply_sender: resp };

        game_task.send_command(cmd).await;

        // Await the response
        let res = resp_rx.await;
        let response = res.unwrap();
        response.user_id
    }
}
