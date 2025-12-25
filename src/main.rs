use anyhow::Result;
use clap::{Parser, Subcommand};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};

mod server;
use server::DVBServer;

#[derive(Parser)]
#[command(name = "dvb-mcp")]
#[command(about = "DVB MCP Server - Dresden public transport assistant", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available resources
    #[command(subcommand)]
    List(ListCommands),
}

#[derive(Subcommand)]
enum ListCommands {
    /// List all available tools
    Tools,
    /// List all available prompts
    Prompts,
    /// List context keys
    Context,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::List(list_cmd)) => {
            let server = DVBServer::default();
            match list_cmd {
                ListCommands::Tools => {
                    server.list_tools();
                }
                ListCommands::Prompts => {
                    server.list_prompts();
                }
                ListCommands::Context => {
                    server.list_context_keys();
                }
            }
            Ok(())
        }
        None => {
            // Default behavior: start the MCP server
            tracing::info!("DVB MCP Server");

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
    }
}
