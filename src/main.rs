use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use rmcp::{ServiceExt, transport::stdio};
use std::io::IsTerminal;
use tracing_subscriber::{self, EnvFilter};

mod server;
use server::DVBServer;

const AFTER_HELP: &str = "\
MCP Server Information:
    This is an MCP (Model Context Protocol) server that automatically detects
    when started by an MCP client (via piped stdin) and enters server mode.

    For MCP Inspector:
      npx @modelcontextprotocol/inspector <path-to-dvb-mcp>

    Note: The 'serve' command is optional - auto-detection handles most cases.";

#[derive(Parser)]
#[command(name = "dvb-mcp")]
#[command(about = "DVB MCP Server - Dresden public transport assistant", long_about = None)]
#[command(version)]
#[command(after_help = AFTER_HELP)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the MCP server (listens on stdin/stdout)
    Serve,

    /// List available resources
    #[command(subcommand)]
    List(ListCommands),

    #[command(hide = true)]
    Version,
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
                ListCommands::Tools => server.list_tools(),
                ListCommands::Prompts => server.list_prompts(),
                ListCommands::Context => server.list_context_keys(),
            }
            Ok(())
        }
        Some(Commands::Serve) => start_mcp_server().await,
        Some(Commands::Version) => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        None => {
            // Check if stdin is a TTY (terminal) or piped from an MCP client
            if std::io::stdin().is_terminal() {
                // Running in terminal without MCP client - show help with MCP context
                let mut cmd = Cli::command();
                let _ = cmd.print_help();
                Ok(())
            } else {
                // stdin is piped - assume MCP client connection
                start_mcp_server().await
            }
        }
    }
}

async fn start_mcp_server() -> Result<()> {
    tracing::info!("DVB MCP Server starting");

    let service = DVBServer::default().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
