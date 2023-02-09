fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute("rusty.GameState", "#[derive(serde::Serialize)]")
        .type_attribute("rusty.Point", "#[derive(serde::Serialize)]")
        .type_attribute("rusty.JoinReply", "#[derive(serde::Serialize)]")
        .compile(&["proto/rusty.proto"], &["proto/"])?;
    Ok(())
}
