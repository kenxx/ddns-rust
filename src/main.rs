mod api;
mod config;
mod provider;

use anyhow::Result;
use clap::Parser;
use log::info;

#[derive(Parser, Debug)]
#[command(name = "ddns-rust")]
#[command(about = "A simple DDNS service supporting multiple DNS providers")]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Load configuration first (before logger init)
    let config = config::Config::load(&args.config)?;

    // Initialize logger with config log level (env var takes precedence)
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&config.server.log_level)
    ).init();

    info!("Loading configuration from: {}", args.config);
    info!(
        "Loaded {} provider(s): {:?}",
        config.providers.len(),
        config.providers.iter().map(|p| &p.name).collect::<Vec<_>>()
    );

    // Create router
    let app = api::create_router(config.clone());

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Server listening on http://{}", addr);
    info!("DDNS endpoint: GET /ddns/{{provider}}/{{host}}/{{ip}}");

    axum::serve(listener, app).await?;

    Ok(())
}
