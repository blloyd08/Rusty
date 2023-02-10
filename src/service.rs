use crate::{
    proto::{
        rusty_server::Rusty, CreateReply, CreateRequest, GameStatusReply, GameStatusRequest,
        JoinReply, JoinRequest, StartReply, StartRequest, UpdateReply, UpdateRequest,
    },
    types::Direction,
    GameError, GameState, JoinGameReply, RustyGame,
};
use log::{debug, info};
use tonic::{Code, Request, Response, Status};

#[derive(Default)]
pub struct RustyService {
    rusty_game: RustyGame,
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
                    width: reply.width as u32,
                    height: reply.height as u32,
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
            rusty_game: RustyGame::default(),
        }
    }

    async fn create_game_internal(&self, request: CreateRequest) -> String {
        self.rusty_game
            .create_game(
                request.width as i32,
                request.height as i32,
                request.tick_duration_millis as u64,
            )
            .await
    }

    async fn update_game_internal(&self, request: UpdateRequest) -> Result<GameState, GameError> {
        let direction: Direction = request.move_direction.into();
        self.rusty_game
            .update_game(request.game_id, request.user_id, direction)
            .await
    }

    async fn game_status_internal(
        &self,
        request: GameStatusRequest,
    ) -> Result<GameState, GameError> {
        self.rusty_game
            .game_status(request.game_id, request.user_id)
            .await
    }

    async fn join_game_internal(&self, request: JoinRequest) -> Result<JoinGameReply, GameError> {
        self.rusty_game.join_game(request.game_id).await
    }

    async fn start_game_internal(&self, request: StartRequest) -> Result<(), GameError> {
        self.rusty_game
            .start_game(request.game_id, request.user_id)
            .await
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
            GameError::Internal => Status::new(Code::Internal, "Internal error"),
        }
    }
}
