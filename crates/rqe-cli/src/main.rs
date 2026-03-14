use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use clap::{Parser, Subcommand};
use poe_rqe::store::QueryId;

#[derive(Parser)]
#[command(name = "rqe-cli", about = "CLI for the Reverse Query Engine")]
struct Cli {
    /// RQE server URL
    #[arg(long, env = "RQE_URL", default_value = "http://localhost:8080")]
    url: String,

    /// API key for authenticated endpoints
    #[arg(long, env = "RQE_API_KEY")]
    api_key: Option<String>,

    /// Directory with extracted datc64 files for item parsing
    #[arg(long, env = "POE_DAT_DIR")]
    dat_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check server health
    Health,

    /// Parse a fixture file and show the Entry that would be sent
    Parse {
        /// Path to a Ctrl+Alt+C item text file
        file: PathBuf,
    },

    /// Parse an item file and match it against all stored queries
    Match {
        /// Path to a Ctrl+Alt+C item text file
        file: PathBuf,
    },

    /// Add a reverse query from a JSON file
    AddQuery {
        /// Path to a JSON file with {conditions: [...], labels: [...]}
        file: PathBuf,
    },

    /// Get a stored query by ID
    GetQuery {
        /// Query ID
        id: QueryId,
    },

    /// Delete a stored query by ID
    DeleteQuery {
        /// Query ID
        id: QueryId,
    },

    /// Match raw Entry JSON against stored queries (no item parsing)
    MatchRaw {
        /// Path to an Entry JSON file (flat key-value map)
        file: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rqe_cli=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let client = poe_rqe_client::RqeClient::new(&cli.url, cli.api_key.clone());

    match &cli.command {
        Command::Health => cmd_health(&client).await?,
        Command::Parse { file } => cmd_parse(&cli, file)?,
        Command::Match { file } => cmd_match(&cli, &client, file).await?,
        Command::AddQuery { file } => cmd_add_query(&client, file).await?,
        Command::GetQuery { id } => cmd_get_query(&client, *id).await?,
        Command::DeleteQuery { id } => cmd_delete_query(&client, *id).await?,
        Command::MatchRaw { file } => cmd_match_raw(&client, file).await?,
    }

    Ok(())
}

async fn cmd_health(client: &poe_rqe_client::RqeClient) -> Result<(), Box<dyn std::error::Error>> {
    let health = client.health().await?;
    println!("Status:      {}", health.status);
    println!("Queries:     {}", health.query_count);
    println!("DAG nodes:   {}", health.node_count);
    Ok(())
}

fn load_game_data(cli: &Cli) -> Result<Arc<poe_data::GameData>, Box<dyn std::error::Error>> {
    let dat_dir = cli.dat_dir.clone().unwrap_or_else(|| {
        let temp = std::env::temp_dir();
        temp.join("poe-dat")
    });

    if !dat_dir.exists() {
        return Err(format!(
            "dat directory not found: {}. Set --dat-dir or POE_DAT_DIR.",
            dat_dir.display()
        )
        .into());
    }

    let gd = poe_data::load(&dat_dir)?;
    Ok(Arc::new(gd))
}

fn parse_item_file(
    cli: &Cli,
    file: &PathBuf,
) -> Result<poe_rqe::eval::Entry, Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let text = std::fs::read_to_string(file)?;
    let raw = poe_item::parse(&text)?;
    let parse_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let gd = load_game_data(cli)?;
    let load_ms = t1.elapsed().as_secs_f64() * 1000.0;

    let t2 = Instant::now();
    let resolved = poe_item::resolve(&raw, &gd);
    let resolve_ms = t2.elapsed().as_secs_f64() * 1000.0;

    let t3 = Instant::now();
    let entry = poe_rqe_client::item_to_entry(&resolved);
    let convert_ms = t3.elapsed().as_secs_f64() * 1000.0;

    eprintln!(
        "  parse: {parse_ms:.1}ms | load_data: {load_ms:.0}ms | resolve: {resolve_ms:.1}ms | convert: {convert_ms:.1}ms | total: {:.0}ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );

    Ok(entry)
}

fn cmd_parse(cli: &Cli, file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let entry = parse_item_file(cli, file)?;
    let json = serde_json::to_string_pretty(&entry)?;
    println!("{json}");
    Ok(())
}

async fn cmd_match(
    cli: &Cli,
    client: &poe_rqe_client::RqeClient,
    file: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let entry = parse_item_file(cli, file)?;

    let t0 = Instant::now();
    let result = client.match_item(&entry).await?;
    let roundtrip_ms = t0.elapsed().as_secs_f64() * 1000.0;

    println!(
        "Matched {} of {} stored queries",
        result.matches.len(),
        result.query_count
    );
    eprintln!(
        "  server match: {}μs | roundtrip: {roundtrip_ms:.1}ms",
        result.match_us
    );
    for m in &result.matches {
        let owner_str = m.owner.as_deref().unwrap_or("anonymous");
        println!("  query {} (owner: {owner_str})", m.id);
    }
    Ok(())
}

async fn cmd_add_query(
    client: &poe_rqe_client::RqeClient,
    file: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(file)?;

    // Support both formats: bare conditions array or {conditions, labels} object
    let parsed: serde_json::Value = serde_json::from_str(&text)?;

    let (conditions, labels, owner) = if parsed.is_array() {
        let conditions: Vec<poe_rqe::predicate::Condition> = serde_json::from_value(parsed)?;
        (conditions, vec![], None)
    } else {
        let conditions: Vec<poe_rqe::predicate::Condition> = serde_json::from_value(
            parsed
                .get("conditions")
                .ok_or("missing 'conditions' field")?
                .clone(),
        )?;
        let labels: Vec<String> = parsed
            .get("labels")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let owner: Option<String> = parsed
            .get("owner")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        (conditions, labels, owner)
    };

    let id = client.add_query(conditions, labels, owner).await?;
    println!("Query added with ID: {id}");
    Ok(())
}

async fn cmd_get_query(
    client: &poe_rqe_client::RqeClient,
    id: QueryId,
) -> Result<(), Box<dyn std::error::Error>> {
    let query = client.get_query(id).await?;
    let json = serde_json::to_string_pretty(&query)?;
    println!("{json}");
    Ok(())
}

async fn cmd_delete_query(
    client: &poe_rqe_client::RqeClient,
    id: QueryId,
) -> Result<(), Box<dyn std::error::Error>> {
    if client.delete_query(id).await? {
        println!("Query {id} deleted");
    } else {
        println!("Query {id} not found");
    }
    Ok(())
}

async fn cmd_match_raw(
    client: &poe_rqe_client::RqeClient,
    file: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(file)?;
    let entry: poe_rqe::eval::Entry = serde_json::from_str(&text)?;
    let result = client.match_item(&entry).await?;

    println!(
        "Matched {} of {} stored queries",
        result.matches.len(),
        result.query_count
    );
    for m in &result.matches {
        let owner_str = m.owner.as_deref().unwrap_or("anonymous");
        println!("  query {} (owner: {owner_str})", m.id);
    }
    Ok(())
}
