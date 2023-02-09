use rusty_game::proto::rusty_server::RustyServer;
use rusty_game::service::RustyService;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();
    let rusty = RustyService::new();

    println!("RustyServer listening on {}", addr);

    Server::builder()
        .add_service(RustyServer::new(rusty))
        .serve(addr)
        .await?;

    Ok(())
}
