use std::io::stdin;
use std::time::Duration;

use rusty::rusty_client::RustyClient;
use rusty::{
    CreateRequest, GameState as ProtoGameState, GameStatusRequest, JoinRequest,
    Point as ProtoPoint, StartRequest, UpdateRequest,
};
use rusty_game::output::print_world;
use rusty_game::proto::MoveDirection;
use rusty_game::{GameState, Point};
use tokio::task::JoinHandle;
use tokio::time;
use tonic::Status;

pub mod rusty {
    tonic::include_proto!("rusty");
}

const WORLD_SIZE: i32 = 10;

#[derive(Debug)]
enum UserInputOption {
    Direction(MoveDirection),
    Exit,
    Retry,
}

impl From<ProtoGameState> for GameState {
    fn from(game_state: ProtoGameState) -> Self {
        Self {
            height: WORLD_SIZE,
            width: WORLD_SIZE,
            tick: 1000,
            game_over_reason: None,
            direction: game_state.move_direction.into(),
            num_users: game_state.number_of_players,
            body: game_state.body.into_iter().map(|p| p.into()).collect(),
            food: game_state.food.unwrap().into(),
        }
    }
}

impl From<ProtoPoint> for Point {
    fn from(value: ProtoPoint) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating Game");
    let game_id = create_game().await;
    println!("Joining Game");
    let user_id = join_game(game_id.clone()).await;
    println!("Starting Game");
    start_game(game_id.clone(), user_id.clone()).await;
    println!("Spawning Tick");
    let _tick_handle = spawn_ticker(game_id.clone(), user_id.clone());

    while let Ok(direction) = get_user_direction() {
        let result = update_game(game_id.clone(), user_id.clone(), direction).await;
        print_world(&result.unwrap().into());
    }

    Ok(())
}

fn spawn_ticker(game_id: String, user_id: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(100));

        loop {
            interval.tick().await;
            match game_status(game_id.clone(), user_id.clone()).await {
                Ok(game_state) => print_world(&game_state.into()),
                Err(err) => {
                    println!("Tick exiting due to error: {}", err);
                    break;
                }
            }
        }
    })
}

fn get_user_direction() -> Result<MoveDirection, ()> {
    let mut user_input_option = UserInputOption::Retry;
    while matches!(user_input_option, UserInputOption::Retry) {
        let mut user_input = String::new();
        println!("What direction do you want to move? (WASD) q=exit");
        let _ = stdin().read_line(&mut user_input);
        let formatted_input = user_input.trim().to_lowercase();
        println!("Input: {:?}", formatted_input);
        user_input_option = match formatted_input.as_str() {
            "w" => UserInputOption::Direction(MoveDirection::North),
            "d" => UserInputOption::Direction(MoveDirection::East),
            "s" => UserInputOption::Direction(MoveDirection::South),
            "a" => UserInputOption::Direction(MoveDirection::West),
            "q" | "e" => UserInputOption::Exit,
            _ => UserInputOption::Retry,
        };
        println!("Input: {:?}", user_input_option);
    }
    let direction_result = match user_input_option {
        UserInputOption::Direction(direction) => Ok(direction),
        _ => Err(()),
    };
    direction_result
}

async fn create_game() -> String {
    let mut client = RustyClient::connect("http://[::1]:50051").await.unwrap();
    let request = tonic::Request::new(CreateRequest {
        height: WORLD_SIZE.try_into().unwrap(),
        width: WORLD_SIZE.try_into().unwrap(),
        tick_duration_millis: 500,
    });

    let response = client.create(request).await.unwrap();

    println!("RESPONSE={:?}", response);
    response.into_inner().game_id
}

async fn join_game(game_id: String) -> String {
    let mut client = RustyClient::connect("http://[::1]:50051").await.unwrap();

    let request = tonic::Request::new(JoinRequest { game_id });

    let response = client.join(request).await.unwrap();

    println!("RESPONSE={:?}", response);
    response.into_inner().user_id
}

async fn start_game(game_id: String, user_id: String) {
    let mut client = RustyClient::connect("http://[::1]:50051").await.unwrap();

    let request = tonic::Request::new(StartRequest { game_id, user_id });

    let response = client.start(request).await.unwrap();

    println!("RESPONSE={:?}", response);
}

async fn update_game(
    game_id: String,
    user_id: String,
    direction: MoveDirection,
) -> Result<ProtoGameState, Status> {
    let mut client = RustyClient::connect("http://[::1]:50051").await.unwrap();

    let request = tonic::Request::new(UpdateRequest {
        game_id,
        user_id,
        move_direction: direction.into(),
    });

    match client.update(request).await {
        Ok(update_reply) => {
            return Ok(update_reply.into_inner().game_state.unwrap());
        }
        Err(err) => {
            println!("Error: {:?}", err);
            return Err(err);
        }
    }
}

async fn game_status(game_id: String, user_id: String) -> Result<ProtoGameState, Status> {
    let mut client = RustyClient::connect("http://[::1]:50051").await.unwrap();

    let request = tonic::Request::new(GameStatusRequest { game_id, user_id });

    match client.game_status(request).await {
        Ok(game_status_reply) => {
            return Ok(game_status_reply.into_inner().game_state.unwrap());
        }
        Err(err) => {
            println!("Error: {:?}", err);
            return Err(err);
        }
    }
}
