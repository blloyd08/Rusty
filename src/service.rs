use log::{debug, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tonic::{Code, Request, Response, Status};
use uuid::Uuid;

use crate::{
    proto::{
        rusty_server::Rusty, CreateReply, CreateRequest, GameStatusReply, GameStatusRequest,
        JoinReply, JoinRequest, StartReply, StartRequest, UpdateReply, UpdateRequest,
    },
    task::{
        GameCommand, GameStatusInternalRequest, GameTask, JoinGameRequest, StartGameRequest,
        UpdateGameRequest, JoinGameReplyInternal,
    },
    types::{Direction, GameError},
    GameState,
};

#[derive(Default)]
pub struct RustyService {
    games: Arc<Mutex<HashMap<String, GameTask>>>,
}

#[tonic::async_trait]
impl Rusty for RustyService {
    async fn create(
        &self,
        request: Request<CreateRequest>,
    ) -> Result<Response<CreateReply>, Status> {
        info!("Received Create request from {:?}", request.remote_addr());

        let game_id = &self.create_game_internal(request.into_inner()).await;

        let reply = CreateReply {
            game_id: game_id.into(),
        };
        Ok(Response::new(reply))
    }

    async fn update(
        &self,
        request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateReply>, Status> {
        debug!("Received update request from {:?}", request.remote_addr());
        match self.update_game_internal(request.into_inner()).await {
            Err(game_error) => Err(Self::convert_game_error_to_status(&game_error)),
            Ok(game_state) => {
                println!("Game State: {:?}", game_state);

                let reply = UpdateReply {
                    game_state: Some(game_state.into()),
                };
                Ok(Response::new(reply))
            }
        }
    }

    async fn join(&self, request: Request<JoinRequest>) -> Result<Response<JoinReply>, Status> {
        info!("Received join request from {:?}", request.remote_addr());
        match self.join_game_internal(request.into_inner()).await {
            Err(game_error) => Err(Self::convert_game_error_to_status(&game_error)),
            Ok(reply) => {
                let reply = JoinReply {
                    user_id: reply.user_id,
                    width: reply.width,
                    height: reply.height,
                };
                Ok(Response::new(reply))
            }
        }
    }

    async fn start(&self, request: Request<StartRequest>) -> Result<Response<StartReply>, Status> {
        info!("Recieved start request from {:?}", request.remote_addr());
        match self.start_game_internal(request.into_inner()).await {
            Err(game_error) => Err(Self::convert_game_error_to_status(&game_error)),
            Ok(_) => {
                let reply = StartReply {};
                Ok(Response::new(reply))
            }
        }
    }

    async fn game_status(
        &self,
        request: Request<GameStatusRequest>,
    ) -> Result<Response<GameStatusReply>, Status> {
        debug!("Received status request from {:?}", request.remote_addr());
        match self.game_status_internal(request.into_inner()).await {
            Err(game_error) => Err(Self::convert_game_error_to_status(&game_error)),
            Ok(game_state) => {
                println!("Game State: {:?}", game_state);

                let reply = GameStatusReply {
                    game_state: Some(game_state.into()),
                };
                Ok(Response::new(reply))
            }
        }
    }
}

impl RustyService {
    pub fn new() -> Self {
        env_logger::init();
        Self {
            games: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn create_game_internal(&self, request: CreateRequest) -> String {
        let game = GameTask::new(request);
        let game_id = Uuid::new_v4().to_string();
        let mut games = self.games.lock().await;
        games.insert(game_id.clone(), game);
        game_id
    }

    async fn update_game_internal(&self, request: UpdateRequest) -> Result<GameState, GameError> {
        let games = self.games.lock().await;
        match games.get(&request.game_id) {
            None => Err(GameError::InvalidGame),
            Some(game) => {
                let (resp_tx, resp_rx) = oneshot::channel();
                let direction: Direction = request.move_direction.into();

                let update_game_request =
                    UpdateGameRequest::new(request.user_id, direction, resp_tx);
                let cmd = GameCommand::UpdateGame {
                    request: update_game_request,
                };
                game.send_command(cmd).await;

                // Await the response
                let res = resp_rx.await;
                println!("GOT = {:?}", res);
                Ok(res.unwrap())
            }
        }
    }

    async fn game_status_internal(
        &self,
        request: GameStatusRequest,
    ) -> Result<GameState, GameError> {
        let games = self.games.lock().await;
        match games.get(&request.game_id) {
            None => Err(GameError::InvalidGame),
            Some(game) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                let game_status_request = GameStatusInternalRequest::new(request.user_id, resp_tx);
                let cmd = GameCommand::GameStatus {
                    request: game_status_request,
                };
                game.send_command(cmd).await;

                // Await the response
                let res = resp_rx.await;
                println!("GOT = {:?}", res);
                Ok(res.unwrap())
            }
        }
    }

    async fn join_game_internal(&self, request: JoinRequest) -> Result<JoinGameReplyInternal, GameError> {
        match self.games.lock().await.get(&request.game_id) {
            None => Err(GameError::InvalidGame),
            Some(game) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                let cmd = GameCommand::JoinGame {
                    request: JoinGameRequest::new(resp_tx),
                };
                game.send_command(cmd).await;

                // Await the response
                let res = resp_rx.await;
                println!("GOT = {:?}", res);
                Ok(res.unwrap())
            }
        }
    }

    async fn start_game_internal(&self, request: StartRequest) -> Result<String, GameError> {
        match self.games.lock().await.get(&request.game_id) {
            None => Err(GameError::InvalidGame),
            Some(game) => {
                let (resp_tx, resp_rx) = oneshot::channel();

                let cmd = GameCommand::StartGame {
                    request: StartGameRequest::new(request.user_id, resp_tx),
                };
                game.send_command(cmd).await;

                // Await the response
                let res = resp_rx.await;
                println!("GOT = {:?}", res);
                Ok(res.unwrap())
            }
        }
    }

    fn convert_game_error_to_status(error: &GameError) -> Status {
        match error {
            GameError::InvalidGame => Status::new(
                Code::InvalidArgument,
                "Invalid Game ID. Create a game first.",
            ),
            GameError::InvalidUser => {
                Status::new(Code::InvalidArgument, "Invalid User ID. Join a game first.")
            }
        }
    }
}
