use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};

mod server;
use server::DVBServer;
mod route_cache;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("DVB MCP Server ");

    // Get current executable path for Inspector
    let current_exe = std::env::current_exe().map(|path| path.display().to_string())?;

    tracing::info!("To test with MCP Inspector:");
    tracing::info!("1. Run: npx @modelcontextprotocol/inspector");
    tracing::info!("2. Enter server command: {}", current_exe);

    let service = DVBServer::default().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
