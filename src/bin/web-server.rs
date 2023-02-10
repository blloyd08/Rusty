// cargo run --bin web-server
#[macro_use]
extern crate rocket;
use std::time::Duration;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};
use rusty::rusty_client::RustyClient;
use rusty::{
    CreateRequest, GameState as ProtoGameState, GameStatusRequest, JoinRequest, StartRequest,
    UpdateRequest,
};
use rusty_game::proto::MoveDirection;
use serde_json::json;
use tokio::time::sleep;
use tonic::Status;

pub mod rusty {
    tonic::include_proto!("rusty");
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/delay/<seconds>")]
async fn delay(seconds: u64) -> String {
    sleep(Duration::from_secs(seconds)).await;
    format!("Waited for {} seconds", seconds)
}

#[get("/create/<height>/<width>/<tick>")]
async fn create(height: u32, width: u32, tick: u32) -> String {
    create_game(height, width, tick).await
}

#[get("/join/<game_id>")]
async fn join(game_id: &str) -> String {
    join_game(game_id.to_string()).await
}

#[get("/start/<game_id>/<user_id>")]
async fn start(game_id: &str, user_id: &str) -> String {
    start_game(game_id.to_string(), user_id.to_string()).await;
    "Done".to_owned()
}

#[get("/update/<game_id>/<user_id>/<direction>")]
async fn update(game_id: &str, user_id: &str, direction: u32) -> String {
    if direction > 3 {
        return "Direction should be a number from 0 to 3.\n0=North, 1=East, 2=South, 3=West"
            .to_string();
    }

    let selected_direction = match direction {
        0 => MoveDirection::North,
        1 => MoveDirection::East,
        2 => MoveDirection::South,
        _ => MoveDirection::West,
    };

    let game_state_response =
        update_game(game_id.to_string(), user_id.to_string(), selected_direction).await;
    let response = format!("{:?}", game_state_response);
    let json_response = json!({
        "error": game_state_response.is_err(),
        "response": response
    });
    json_response.to_string()
}

#[get("/status/<game_id>/<user_id>")]
async fn status(game_id: &str, user_id: &str) -> String {
    let game_state_response = game_status(game_id.to_string(), user_id.to_string()).await;
    match game_state_response {
        Ok(game_state) => json!({
            "error": false,
            "response": game_state
        })
        .to_string(),
        Err(err) => json!({
            "error": true,
            "response": err.to_string()
        })
        .to_string(),
    }
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _rocket = rocket::build()
        .mount(
            "/",
            routes![index, delay, create, join, status, update, start,],
        )
        .attach(CORS)
        .launch()
        .await?;
    Ok(())
}

async fn create_game(height: u32, width: u32, tick: u32) -> String {
    let mut client = RustyClient::connect("http://[::1]:50051").await.unwrap();
    let request = tonic::Request::new(CreateRequest {
        height: height,
        width: width,
        tick_duration_millis: tick,
    });

    let response = client.create(request).await.unwrap();

    println!("RESPONSE={:?}", response);
    response.into_inner().game_id
}

async fn join_game(game_id: String) -> String {
    let mut client = RustyClient::connect("http://[::1]:50051").await.unwrap();

    let request = tonic::Request::new(JoinRequest { game_id });

    let response = client.join(request).await.unwrap();

    let json_response = json!({
        "error": 0,
        "response": response.into_inner()
    });
    json_response.to_string()
}

async fn start_game(game_id: String, user_id: String) {
    let mut client = RustyClient::connect("http://[::1]:50051").await.unwrap();

    let request = tonic::Request::new(StartRequest { game_id, user_id });

    let _response = client.start(request).await.unwrap();
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
