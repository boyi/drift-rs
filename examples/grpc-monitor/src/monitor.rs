use std::time::Duration;

use drift_rs::{
    math::constants::{BASE_PRECISION_U64, PRICE_PRECISION_U64},
    types::{
        accounts::User,
        MarketId,
    },
    Context, DriftClient, GrpcSubscribeOpts, RpcClient, Wallet,
};
use solana_sdk::commitment_config::CommitmentLevel;

use crate::display;

/// Monitor state to track changes
#[derive(Default)]
struct MonitorState {
    btc_perp_price: Option<f64>,
    usdc_balance: Option<i64>,
    jlp_balance: Option<i64>,
    btc_position_size: Option<i64>,
    btc_position_pnl: Option<i128>,
}

impl MonitorState {
    fn update_btc_price(&mut self, new_price: f64, threshold: Option<f64>) -> bool {
        let changed = if let Some(old_price) = self.btc_perp_price {
            if let Some(threshold_value) = threshold {
                // Only show changes above threshold
                let change_pct = (new_price - old_price).abs() / old_price;
                change_pct >= threshold_value
            } else {
                // Show all price changes (when price actually changes)
                old_price != new_price
            }
        } else {
            true // First update
        };

        if changed {
            if let Some(old_price) = self.btc_perp_price {
                display::print_price_update("BTC-PERP", old_price, new_price, PRICE_PRECISION_U64);
            }
            self.btc_perp_price = Some(new_price);
        }

        changed
    }

    fn update_usdc_balance(&mut self, new_balance: i64) -> bool {
        let changed = self.usdc_balance.map_or(true, |old| old != new_balance);

        if changed {
            if let Some(old_balance) = self.usdc_balance {
                // USDC uses 6 decimal places
                display::print_balance_update("USDC", old_balance, new_balance, 1_000_000);
            }
            self.usdc_balance = Some(new_balance);
        }

        changed
    }

    fn update_jlp_balance(&mut self, new_balance: i64) -> bool {
        let changed = self.jlp_balance.map_or(true, |old| old != new_balance);

        if changed {
            if let Some(old_balance) = self.jlp_balance {
                // JLP uses 6 decimal places
                display::print_balance_update("JLP", old_balance, new_balance, 1_000_000);
            }
            self.jlp_balance = Some(new_balance);
        }

        changed
    }

    fn update_btc_position(&mut self, new_size: i64, new_pnl: i128) -> bool {
        let size_changed = self.btc_position_size.map_or(true, |old| old != new_size);
        let pnl_changed = self.btc_position_pnl.map_or(true, |old| old != new_pnl);
        let changed = size_changed || pnl_changed;

        if changed {
            display::print_position_update("BTC-PERP", new_size, new_pnl as i64, PRICE_PRECISION_U64, BASE_PRECISION_U64);
            self.btc_position_size = Some(new_size);
            self.btc_position_pnl = Some(new_pnl);
        }

        changed
    }
}

pub async fn start_monitoring(
    context: Context,
    wallet: Wallet,
    rpc_url: String,
    grpc_url: String,
    grpc_token: Option<String>,
    price_threshold: Option<f64>,
    sub_account_index: u16,
    mode: String,
    amount: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    display::print_header("BTC-USDC gRPC Monitor");

    // Outer loop for reconnection on failure
    loop {
        match start_monitoring_inner(
            context.clone(),
            wallet.clone(),
            rpc_url.clone(),
            grpc_url.clone(),
            grpc_token.clone(),
            price_threshold,
            sub_account_index,
            mode.clone(),
            amount,
        ).await {
            Ok(_) => {
                display::print_info("Monitor ended normally");
                break;
            }
            Err(e) => {
                display::print_error(&format!("Monitor failed: {}. Reconnecting in 10 seconds...", e));
                tokio::time::sleep(Duration::from_secs(10)).await;
                display::print_info("Attempting to reconnect...");
            }
        }
    }

    Ok(())
}

async fn start_monitoring_inner(
    context: Context,
    wallet: Wallet,
    rpc_url: String,
    grpc_url: String,
    grpc_token: Option<String>,
    price_threshold: Option<f64>,
    sub_account_index: u16,
    mode: String,
    amount: f64,
) -> Result<(), Box<dyn std::error::Error>> {

    // Initialize Drift client
    display::print_info("Initializing Drift client...");
    let rpc_client = RpcClient::new(rpc_url.clone());

    let drift = match DriftClient::new(context, rpc_client, wallet.clone()).await {
        Ok(client) => {
            display::print_success("Drift client initialized");
            client
        },
        Err(e) => {
            display::print_error(&format!("Failed to initialize Drift client: {}", e));
            display::print_info("This might be due to:");
            display::print_info("  - Network mismatch (devnet context with mainnet RPC)");
            display::print_info("  - RPC endpoint issues");
            display::print_info("  - Missing lookup table accounts");
            return Err(e.into());
        }
    };

    // Subscribe to blockhashes for faster transaction building
    drift.subscribe_blockhashes().await?;

    // Get market info
    let btc_perp_market_id = drift
        .market_lookup("btc-perp")
        .ok_or("BTC-PERP market not found")?;

    let usdc_spot_market_id = drift
        .market_lookup("usdc")
        .unwrap_or_else(|| MarketId::spot(0)); // Fallback to index 0

    // JLP market - try to find it
    let jlp_spot_market_id = drift
        .market_lookup("jlp")
        .or_else(|| drift.market_lookup("JLP"))
        .unwrap_or_else(|| MarketId::spot(7)); // JLP is usually market index 7

    display::print_info(&format!(
        "Monitoring BTC-PERP (market {}), USDC spot (market {}), JLP spot (market {})",
        btc_perp_market_id.index(),
        usdc_spot_market_id.index(),
        jlp_spot_market_id.index()
    ));

    // Setup gRPC subscription
    display::print_info("Setting up gRPC subscription...");

    let grpc_opts = GrpcSubscribeOpts::default()
        .commitment(CommitmentLevel::Confirmed)
        .usermap_on(); // Cache all user accounts
        // Note: oraclemap is not available in gRPC opts, we'll subscribe manually

    // Use the new optional token API
    let result = if grpc_token.is_some() {
        drift
            .grpc_subscribe(grpc_url.clone(), grpc_token.unwrap(), grpc_opts, true)
            .await
    } else {
        drift
            .grpc_subscribe_with_optional_token(grpc_url.clone(), None, grpc_opts, true)
            .await
    };

    if let Err(err) = result {
        display::print_error(&format!("gRPC subscription failed: {:?}", err));
        return Err(err.into());
    }

    display::print_success("gRPC subscription active");

    // Get user account address from drift client's wallet
    let user_account = drift.wallet().sub_account(sub_account_index);
    display::print_info(&format!("Monitoring sub-account {}: {}", sub_account_index, user_account));
    display::print_info(&format!("Wallet authority: {}", drift.wallet().authority()));

    // Subscribe to the user account to ensure we get updates
    match drift.subscribe_account(&user_account).await {
        Ok(_) => display::print_success("Subscribed to user account"),
        Err(e) => {
            display::print_error(&format!("Failed to subscribe to user account: {:?}", e));
            display::print_info("Will continue monitoring prices only");
        }
    }

    // Subscribe to oracle updates for BTC market
    match drift.subscribe_oracles(&[btc_perp_market_id]).await {
        Ok(_) => display::print_success("Subscribed to BTC oracle updates"),
        Err(e) => {
            // AlreadySubscribed is fine - it means gRPC already handles it
            if !format!("{:?}", e).contains("AlreadySubscribed") {
                display::print_error(&format!("Failed to subscribe to oracle: {:?}", e));
            } else {
                display::print_info("Oracle already subscribed via gRPC");
            }
        }
    }

    // Wait a moment for the subscription to sync
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Monitor state
    let mut state = MonitorState::default();
    let mut status_timer = tokio::time::interval(Duration::from_secs(30));
    let mut update_timer = tokio::time::interval(Duration::from_millis(100)); // Check every 100ms for more responsive updates

    // Check if user account exists initially
    match drift.try_get_account::<User>(&user_account) {
        Ok(user_data) => {
            display::print_success(&format!("User account found with {} spot positions and {} perp positions",
                user_data.spot_positions.len(), user_data.perp_positions.len()));

            // Check initial USDC balance
            match user_data.get_spot_position(usdc_spot_market_id.index()) {
                Ok(spot_position) => {
                    // Get the spot market account for USDC to calculate actual token amount
                    if let Ok(usdc_market) = drift.try_get_spot_market_account(usdc_spot_market_id.index()) {
                        if let Ok(token_amount) = spot_position.get_token_amount(&usdc_market) {
                            // USDC has 6 decimals
                            display::print_info(&format!("Initial USDC balance: {:.6}",
                                token_amount as f64 / 1_000_000.0));
                        }
                    }
                }
                Err(_) => {
                    display::print_info("No USDC spot position found");
                }
            }

            // Check initial JLP balance
            match user_data.get_spot_position(jlp_spot_market_id.index()) {
                Ok(spot_position) => {
                    // Get the spot market account for JLP to calculate actual token amount
                    if let Ok(jlp_market) = drift.try_get_spot_market_account(jlp_spot_market_id.index()) {
                        if let Ok(token_amount) = spot_position.get_token_amount(&jlp_market) {
                            // JLP has 6 decimals
                            display::print_info(&format!("Initial JLP balance: {:.6}",
                                token_amount as f64 / 1_000_000.0));
                        }
                    }
                }
                Err(_) => {
                    display::print_info("No JLP spot position found");
                }
            }

            // Check initial BTC position
            match user_data.get_perp_position(btc_perp_market_id.index()) {
                Ok(perp_position) => {
                    display::print_info(&format!("Initial BTC position size: {:.6}",
                        perp_position.base_asset_amount as f64 / 1_000_000_000.0));
                }
                Err(_) => {
                    display::print_info("No BTC perp position found");
                }
            }
        }
        Err(e) => {
            display::print_error(&format!("User account not found: {:?}", e));
            display::print_info("This account hasn't been initialized with Drift yet");
            display::print_info("You can still monitor prices, but balance/position data won't be available");
            display::print_info("To initialize: deposit funds or place a trade on Drift Protocol");
        }
    }

    display::print_success("Starting real-time monitoring...");
    display::print_divider();

    // Execute trading mode if specified
    if mode != "monitor" {
        display::print_info(&format!("⏱️ Waiting 5 seconds before executing {} mode...", mode));
        tokio::time::sleep(Duration::from_secs(5)).await;

        let signature = match mode.as_str() {
            "swap-jlp" => {
                display::print_header("Executing JLP Swap");
                match crate::trading::buy_jlp_via_jupiter(&drift, amount).await {
                    Ok(sig) => sig,
                    Err(e) => {
                        display::print_error(&format!("Failed to execute JLP swap: {}", e));
                        return Err(e);
                    }
                }
            }
            "buy-btc" => {
                display::print_header("Executing BTC-PERP Buy Order");
                match crate::trading::buy_btc_perp(&drift, amount).await {
                    Ok(sig) => sig,
                    Err(e) => {
                        display::print_error(&format!("Failed to execute BTC buy order: {}", e));
                        return Err(e);
                    }
                }
            }
            _ => unreachable!()
        };

        // Monitor transaction status
        display::print_divider();
        display::print_info("Monitoring transaction status...");
        match crate::trading::monitor_transaction(&drift, &signature, 60, &rpc_url).await {
            Ok(true) => {
                display::print_success("Transaction confirmed successfully!");
                display::print_info("Continuing to monitor balance changes...");
            }
            Ok(false) => {
                display::print_error("Transaction failed or timed out");
            }
            Err(e) => {
                display::print_error(&format!("Error monitoring transaction: {}", e));
            }
        }
        display::print_divider();

        // Wait a bit for balances to update
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    loop {
        tokio::select! {
            _ = status_timer.tick() => {
                // Print status summary every 30 seconds
                display::print_status_summary(
                    state.btc_perp_price,
                    state.usdc_balance,
                    state.jlp_balance,
                    state.btc_position_size,
                    state.btc_position_pnl,
                );
            }
            _ = update_timer.tick() => {
                // Check for updates every second

                // Check BTC price
                if let Some(oracle_data) = drift.try_get_oracle_price_data_and_slot(btc_perp_market_id) {
                    let price = oracle_data.data.price as f64;
                    state.update_btc_price(price, price_threshold);
                }

                // Check user account updates
                match drift.try_get_account::<User>(&user_account) {
                    Ok(user_data) => {
                        // Check USDC balance
                        match user_data.get_spot_position(usdc_spot_market_id.index()) {
                            Ok(spot_position) => {
                                // Get the spot market account for USDC to calculate actual token amount
                                if let Ok(usdc_market) = drift.try_get_spot_market_account(usdc_spot_market_id.index()) {
                                    if let Ok(token_amount) = spot_position.get_token_amount(&usdc_market) {
                                        state.update_usdc_balance(token_amount as i64);
                                    }
                                }
                            }
                            Err(_) => {
                                // User might not have USDC position yet, this is normal
                            }
                        }

                        // Check JLP balance
                        match user_data.get_spot_position(jlp_spot_market_id.index()) {
                            Ok(spot_position) => {
                                // Get the spot market account for JLP to calculate actual token amount
                                if let Ok(jlp_market) = drift.try_get_spot_market_account(jlp_spot_market_id.index()) {
                                    if let Ok(token_amount) = spot_position.get_token_amount(&jlp_market) {
                                        state.update_jlp_balance(token_amount as i64);
                                    }
                                }
                            }
                            Err(_) => {
                                // User might not have JLP position yet, this is normal
                            }
                        }

                        // Check BTC position
                        match user_data.get_perp_position(btc_perp_market_id.index()) {
                            Ok(perp_position) => {
                                let size = perp_position.base_asset_amount;

                                // Calculate unrealized PnL
                                let pnl = if let Some(oracle_data) = drift.try_get_oracle_price_data_and_slot(btc_perp_market_id) {
                                    perp_position.get_unrealized_pnl(oracle_data.data.price).unwrap_or(0)
                                } else {
                                    0
                                };

                                state.update_btc_position(size, pnl);
                            }
                            Err(_) => {
                                // User might not have BTC position yet, this is normal
                            }
                        }
                    }
                    Err(e) => {
                        // Only print error occasionally to avoid spam
                        if update_timer.period().as_secs() % 10 == 0 {
                            display::print_error(&format!("Failed to get user account: {:?}", e));
                        }
                    }
                }
            }
        }
    }
}