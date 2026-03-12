//! RSK MCP Server Entry Point
//!
//! Exposes RSK's microgram runtime, statistics, decision engine,
//! and graph operations to AI agents via Model Context Protocol.

use anyhow::Result;
use rsk_mcp::RskMcpServer;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting RSK MCP server");

    let server = RskMcpServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
