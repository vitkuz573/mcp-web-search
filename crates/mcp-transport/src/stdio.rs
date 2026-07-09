use tracing::info;

pub async fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting MCP server on stdio transport");
    info!("stdio transport ready, waiting for connections...");
    Ok(())
}
