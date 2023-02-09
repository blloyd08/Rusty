use log::{info, warn};
use std::{sync::Arc, sync::Weak};
use std::time::Duration;

use tokio::{
    sync::{
        mpsc::{self, Sender},
        Mutex,
    },
    task::JoinHandle,
    time::{self},
};
use uuid::Uuid;

use crate::{
    game::{Game, SharedGame},
    output::print_world,
    proto::CreateRequest,
    types::Direction,
    GameState, Responder,
};

pub(crate) struct GameTask {
    _manager: JoinHandle<()>,
    sender: Arc<Sender<GameCommand>>,
}

#[derive(Debug)]
pub struct JoinGameReplyInternal {
    pub user_id: String,
    pub width: u32,
    pub height: u32,
}

impl GameTask {
    pub fn new(request: CreateRequest) -> Self {
        let (tx, mut rx) = mpsc::channel::<GameCommand>(32);
        let sender = Arc::new(tx);
        let weak_game_sender = Arc::downgrade(&sender);

        // The `move` keyword is used to **move** ownership of `rx` into the task.
        let _manager = tokio::spawn(async move {
            let game_sender = weak_game_sender;
            let height: i32 = request.height.try_into().unwrap();
            let width: i32 = request.width.try_into().unwrap();
            let max_spaces: usize = (width * height).try_into().unwrap();
            let tick_duration_millis: u64 = request.tick_duration_millis.try_into().unwrap();
            let shared_game = Arc::new(Mutex::new(Game::new(height, width)));
            let mut _tick_handle = None;
            // Start receiving messages
            while let Some(cmd) = rx.recv().await {
                use GameCommand::*;

                match cmd {
                    GameStatus { request } => {
                        GameTask::game_status(request, shared_game.clone()).await;
                    }
                    UpdateGame { request } => {
                        GameTask::update_game(request, shared_game.clone()).await;
                    }
                    JoinGame { request } => {
                        GameTask::join_game(request, shared_game.clone()).await;
                    }
                    StartGame { request } => {
                        match GameTask::start_game(
                            request,
                            shared_game.clone(),
                            tick_duration_millis,
                            game_sender.clone(),
                        ).await {
                            Ok(tick_handle) => {
                                _tick_handle = Some(tick_handle);
                            },
                            Err(_) => {}
                        }
                    }
                    Tick {} => {
                        println!("Tick received");
                        let game_state = GameTask::tick(shared_game.clone(), max_spaces).await;
                        let game_over = game_state.game_over_reason.is_some();
                        print_world(&game_state);
                        if game_over {
                            break;
                        }
                    }
                }
            }
            warn!("Exiting game loop");
        });

        Self {
            _manager,
            sender,
        }
    }

    pub async fn send_command(&self, command: GameCommand) {
        if let Err(error) = self.sender.send(command).await {
            print!("{}", error);
            todo!();
        }
    }

    async fn game_status(request: GameStatusInternalRequest, shared_game: SharedGame) {
        // TODO: Verify user has joined
        let game = shared_game.lock().await;
        let game_state = game.into_game_state().await;

        // Ignore errors
        let _ = request.resp.send(game_state);
    }

    async fn update_game(request: UpdateGameRequest, shared_game: SharedGame) {
        // TODO: Verify user has registered
        let game = shared_game.lock().await;
        game.add_user_direction(request.user_id, request.direction)
            .await;

        let game_state = game.into_game_state().await;
        // Ignore errors
        let _ = request.resp.send(game_state);
    }

    async fn join_game(request: JoinGameRequest, shared_game: SharedGame) {
        let user_id = Uuid::new_v4().to_string();
        let game = shared_game.lock().await;
        let _user_is_added = game.add_user(user_id.clone()).await;
        let (width, height) = game.get_dimensions();

        // Ignore errors
        let _ = request.resp.send(JoinGameReplyInternal {
            user_id,
            width,
            height
        });
    }

    async fn start_game(
        request: StartGameRequest,
        shared_game: SharedGame,
        tick_duration_millis: u64,
        command_sender: Weak<Sender<GameCommand>>,
    ) -> Result<JoinHandle<()>, String> {
        let game = shared_game.lock().await;
        if game.user_has_joined_game(request.user_id).await {
            let _tick = tokio::spawn(async move {
                let mut interval =
                    time::interval(Duration::from_millis(tick_duration_millis));
                loop {
                    interval.tick().await;
                    if let Some(tick_sender) = command_sender.upgrade() {
                        match tick_sender.send(GameCommand::Tick {}).await {
                            Ok(_) => info!("Tick!"),
                            Err(_) => {
                                warn!("Failed to send tick. Channel Closed");
                                break;
                            },
                        }
                    } else {
                        warn!("Command sender dropped. Exiting tick loop");
                        break;
                    }
                }
            });
            return Ok(_tick);
        }
        Err("Must join game before starting game".to_owned())
    }

    async fn tick(shared_game: SharedGame, max_spaces: usize) -> GameState {
        let mut game = shared_game.lock().await;
        game.tick(max_spaces).await;
        game.into_game_state().await
    }
}

#[derive(Debug)]
pub(crate) struct UpdateGameRequest {
    user_id: String,
    direction: Direction,
    resp: Responder<GameState>,
}

impl UpdateGameRequest {
    pub fn new(user_id: String, direction: Direction, resp: Responder<GameState>) -> Self {
        Self {
            user_id,
            direction,
            resp,
        }
    }
}

#[derive(Debug)]
pub(crate) struct GameStatusInternalRequest {
    user_id: String,
    resp: Responder<GameState>,
}

impl GameStatusInternalRequest {
    pub fn new(user_id: String, resp: Responder<GameState>) -> Self {
        Self { user_id, resp }
    }
}

#[derive(Debug)]
pub(crate) struct JoinGameRequest {
    resp: Responder<JoinGameReplyInternal>,
}

impl JoinGameRequest {
    pub fn new(resp: Responder<JoinGameReplyInternal>) -> Self {
        Self { resp }
    }
}

#[derive(Debug)]
pub(crate) struct StartGameRequest {
    user_id: String,
    resp: Responder<String>,
}

impl StartGameRequest {
    pub fn new(user_id: String, resp: Responder<String>) -> Self {
        Self { user_id, resp }
    }
}

pub(crate) enum GameCommand {
    UpdateGame { request: UpdateGameRequest },
    GameStatus { request: GameStatusInternalRequest },
    JoinGame { request: JoinGameRequest },
    StartGame { request: StartGameRequest },
    Tick {},
}

#[cfg(test)]
mod tests {
    use crate::output::print_world;
    use crate::proto::CreateRequest;
    use crate::task::GameState;
    use crate::Point;
    use tokio::sync::oneshot::{self};

    use crate::{
        task::{GameCommand, GameTask, JoinGameRequest, UpdateGameRequest},
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
            request: UpdateGameRequest {
                user_id,
                direction: Direction::South,
                resp,
            },
        };

        game_task.send_command(cmd).await;

        // Await the response
        let res = resp_rx.await;
        println!("GOT = {:?}", res);
        let game_state = res.unwrap();
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
        GameTask::new(CreateRequest {
            height: 10,
            width: 10,
            tick_duration_millis: 1000,
        })
    }

    async fn join_game(game_task: &GameTask) -> String {
        let (resp, resp_rx) = oneshot::channel();
        // Send the create game request
        let cmd = GameCommand::JoinGame {
            request: JoinGameRequest { resp },
        };

        game_task.send_command(cmd).await;

        // Await the response
        let res = resp_rx.await;
        println!("Join Game Response = {:?}", res);
        let response = res.unwrap();
        response.user_id
    }
}
