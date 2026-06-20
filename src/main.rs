//! CLI binary for querying Minecraft servers.
//!
//! Usage: `mcstatus [--bedrock | --legacy] <address> [ping|status|query|json]`

use clap::{Parser, Subcommand};

use mcstatus::error::{McStatusError, Result};
use mcstatus::server::{BedrockServer, JavaServer, LegacyServer};

#[derive(Parser)]
#[command(
    name = "mcstatus",
    version = env!("CARGO_PKG_VERSION"),
    about = "Query Minecraft servers for their status and capabilities."
)]
struct Cli {
    /// The address of the server (host[:port]).
    address: String,

    /// Specifies that the address is a Bedrock Edition server.
    #[arg(long, group = "server_type")]
    bedrock: bool,

    /// Specifies that the address is a pre-1.7 Java Edition server.
    #[arg(long, group = "server_type")]
    legacy: bool,

    /// Skip DNS SRV record lookup (use the address as-is).
    #[arg(long)]
    no_srv: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Ping the server for latency.
    Ping,
    /// Print server status (default).
    Status,
    /// Print detailed server information (query protocol, Java only).
    Query,
    /// Print server status and query in JSON.
    Json,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let command = cli.command.unwrap_or(Commands::Status);

    if cli.bedrock {
        run_bedrock(cli.address, command).await
    } else if cli.legacy {
        run_legacy(cli.address, command, cli.no_srv).await
    } else {
        run_java(cli.address, command, cli.no_srv).await
    }
}

async fn run_java(address: String, command: Commands, no_srv: bool) -> Result<()> {
    let server = JavaServer::lookup(&address, 3.0, no_srv).await?;

    match command {
        Commands::Ping => {
            match server.ping().await {
                Ok(latency) => println!("{latency:.2}ms"),
                Err(e) => {
                    // Fallback: try status for latency
                    eprintln!("Warning: Ping failed ({}), falling back to status latency.", e);
                    let status = server.status().await?;
                    println!("{:.2}ms", status.latency);
                }
            }
        }
        Commands::Status => {
            let status = server.status().await?;
            print_java_status(&status);
        }
        Commands::Query => {
            let query = server.query().await?;
            print_query(&query);
        }
        Commands::Json => {
            let status = server.status().await?;
            let query = server.query().await.ok();
            let output = serde_json::json!({
                "status": serde_json::to_value(&status).ok(),
                "query": query.map(|q| serde_json::to_value(&q).unwrap_or_default()),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
    }

    Ok(())
}

async fn run_bedrock(address: String, command: Commands) -> Result<()> {
    let server = BedrockServer::lookup(&address, 3.0).await?;

    match command {
        Commands::Ping | Commands::Status => {
            let status = server.status().await?;
            print_bedrock_status(&status);
        }
        Commands::Query => {
            return Err(McStatusError::Other(
                "The 'query' protocol is only supported by Java servers.".into(),
            ));
        }
        Commands::Json => {
            let status = server.status().await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::to_value(&status).unwrap()).unwrap()
            );
        }
    }

    Ok(())
}

async fn run_legacy(address: String, command: Commands, no_srv: bool) -> Result<()> {
    let server = LegacyServer::lookup(&address, 3.0, no_srv).await?;

    match command {
        Commands::Ping | Commands::Status => {
            let status = server.status().await?;
            print_legacy_status(&status);
        }
        Commands::Query => {
            return Err(McStatusError::Other(
                "The 'query' protocol is only supported by modern Java servers.".into(),
            ));
        }
        Commands::Json => {
            let status = server.status().await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::to_value(&status).unwrap()).unwrap()
            );
        }
    }

    Ok(())
}

fn print_java_status(status: &mcstatus::response::java::JavaStatusResponse) {
    println!("version: {}", status.version.name);
    println!("motd: {}", status.motd.to_ansi());
    println!(
        "players: {}/{}",
        status.players.online, status.players.max
    );
    if let Some(ref sample) = status.players.sample {
        for player in sample {
            println!("  - {} ({})", player.name, player.id);
        }
    }
    println!("latency: {:.2}ms", status.latency);
    if let Some(ref forge) = status.forge_data {
        println!("forge mods: {}", forge.mods.len());
    }
}

fn print_bedrock_status(status: &mcstatus::response::bedrock::BedrockStatusResponse) {
    if let Some(ref name) = status.version.name {
        println!("version: {name}");
    }
    println!("motd: {}", status.motd.to_ansi());
    println!(
        "players: {}/{}",
        status.players.online, status.players.max
    );
    if let Some(ref brand) = status.version.brand {
        println!("brand: {brand}");
    }
    if let Some(ref map) = status.map_name {
        println!("map: {map}");
    }
    if let Some(ref gm) = status.gamemode {
        println!("gamemode: {gm}");
    }
    println!("latency: {:.2}ms", status.latency);
}

fn print_legacy_status(status: &mcstatus::response::legacy::LegacyStatusResponse) {
    println!("version: {}", status.version.name);
    println!("motd: {}", status.motd.to_ansi());
    println!(
        "players: {}/{}",
        status.players.online, status.players.max
    );
    println!("latency: {:.2}ms", status.latency);
}

fn print_query(query: &mcstatus::response::query::QueryResponse) {
    println!("host: {}:{}",
        query.ip.as_deref().unwrap_or("unknown"),
        query.port.unwrap_or(0)
    );
    println!("motd: {}", query.motd.to_plain());
    println!("gametype: {}", query.game_type.as_deref().unwrap_or("unknown"));
    if let Some(ref sw) = query.software {
        if let Some(ref brand) = sw.brand {
            println!("software: {brand}");
        }
        if !sw.plugins.is_empty() {
            println!("plugins: {}", sw.plugins.join(", "));
        }
    }
    println!(
        "players: {}/{}",
        query.players.online, query.players.max
    );
    for player in &query.players.list {
        println!("  - {player}");
    }
}
