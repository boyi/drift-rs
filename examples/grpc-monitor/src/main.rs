use std::path::PathBuf;

use argh::FromArgs;
use drift_rs::{Context, Wallet};
use solana_sdk::signature::Keypair;

mod display;
mod monitor;

/// Real-time gRPC monitoring of BTC prices and USDC balance
#[derive(FromArgs)]
struct Args {
    /// path to wallet JSON file (Solana CLI format or {"private_key": "base58..."})
    #[argh(option, short = 'w')]
    wallet_file: PathBuf,

    /// gRPC endpoint URL (supports HTTP and HTTPS). If not provided, reads from GRPC_URL env var
    #[argh(option)]
    grpc_url: Option<String>,

    /// gRPC X-Token for authentication (optional). If not provided, reads from GRPC_X_TOKEN env var
    #[argh(option)]
    grpc_token: Option<String>,

    /// disable authentication (useful for local testing)
    #[argh(switch)]
    no_auth: bool,

    /// solana RPC endpoint. If not provided, reads from RPC_URL env var
    #[argh(option)]
    rpc_url: Option<String>,

    /// price change threshold for alerts (e.g., 0.01 for 1%). If not set, prints all price changes
    #[argh(option)]
    price_threshold: Option<f64>,

    /// use mainnet (default is devnet)
    #[argh(switch)]
    mainnet: bool,

    /// sub account index to monitor (default is 0)
    #[argh(option)]
    sub_account: Option<u16>,
}


/// Load keypair from JSON file
/// Supports both Solana CLI format ([1,2,3,...]) and custom format ({"private_key": "base58..."})
fn load_keypair_from_file(path: &PathBuf) -> Result<Keypair, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read wallet file '{}': {}", path.display(), e))?;

    // Try to parse as Solana CLI format first (array of numbers)
    if let Ok(bytes) = serde_json::from_str::<Vec<u8>>(&content) {
        if bytes.len() == 64 {
            return Ok(Keypair::try_from(&bytes[..])
                .map_err(|e| format!("Invalid keypair bytes: {}", e))?);
        }
    }

    // Try custom format with private_key field
    #[derive(serde::Deserialize)]
    struct WalletFile {
        private_key: String,
    }

    if let Ok(wallet_data) = serde_json::from_str::<WalletFile>(&content) {
        let keypair = drift_rs::utils::load_keypair_multi_format(&wallet_data.private_key)
            .map_err(|e| format!("Failed to parse private_key: {:?}", e))?;
        return Ok(keypair);
    }

    Err(format!(
        "Unsupported wallet file format. Expected Solana CLI format [1,2,3,...] or {{\"private_key\": \"base58...\"}}"
    ).into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = argh::from_env();
    dotenv::dotenv().ok(); // Load .env if available
    // Load wallet from JSON file
    println!("📁 Loading wallet from: {}", args.wallet_file.display());
    let keypair = load_keypair_from_file(&args.wallet_file)?;
    let wallet = Wallet::from(keypair);

    println!("💰 Wallet address: {}", wallet.authority());

    // Determine context
    let context = if args.mainnet {
        Context::MainNet
    } else {
        Context::DevNet
    };

    println!("🌐 Using {} context", if args.mainnet { "MainNet" } else { "DevNet" });

    // // Warn if there's a potential network mismatch
    // let is_mainnet_rpc = args.rpc_url.contains("mainnet");
    // let is_mainnet_grpc = args.grpc_url.contains("mainnet");

    // if args.mainnet != is_mainnet_rpc {
    //     println!("⚠️  Warning: Context ({}) doesn't match RPC URL ({})",
    //              if args.mainnet { "mainnet" } else { "devnet" },
    //              if is_mainnet_rpc { "mainnet" } else { "devnet" });
    // }

    // if args.mainnet != is_mainnet_grpc {
    //     println!("⚠️  Warning: Context ({}) doesn't match gRPC URL ({})",
    //              if args.mainnet { "mainnet" } else { "devnet" },
    //              if is_mainnet_grpc { "mainnet" } else { "devnet" });
    // }

    // Get URLs from args or environment variables
    let rpc_url = args.rpc_url
        .or_else(|| std::env::var("RPC_URL").ok())
        .unwrap_or_else(|| {
            if args.mainnet {
                "https://api.mainnet-beta.solana.com".to_string()
            } else {
                "https://api.devnet.solana.com".to_string()
            }
        });

    let grpc_url = args.grpc_url
        .or_else(|| std::env::var("GRPC_URL").ok())
        .ok_or("gRPC URL must be provided via --grpc-url argument or GRPC_URL environment variable")?;

    // Determine authentication
    let grpc_token = if args.no_auth {
        None
    } else {
        args.grpc_token.or_else(|| std::env::var("GRPC_X_TOKEN").ok())
    };

    if grpc_token.is_some() {
        println!("🔐 Using authentication token");
    } else {
        println!("🔓 No authentication (useful for local testing)");
    }

    println!("🔗 RPC endpoint: {}", rpc_url);
    println!("🔗 gRPC endpoint: {}", grpc_url);

    if let Some(threshold) = args.price_threshold {
        println!("📊 Price change threshold: {}%", threshold * 100.0);
    } else {
        println!("📊 Price monitoring: All price changes will be displayed");
    }

    println!();

    // Start monitoring
    monitor::start_monitoring(
        context,
        wallet,
        rpc_url,
        grpc_url,
        grpc_token,
        args.price_threshold,
        args.sub_account.unwrap_or(0),
    )
    .await?;

    Ok(())
}