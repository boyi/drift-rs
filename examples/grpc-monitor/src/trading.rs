use std::time::Duration;

use drift_rs::{
    DriftClient,
    Wallet,
    TransactionBuilder,
    types::{
        accounts::User,
        MarketId,
        NewOrder,
    },
    jupiter::{JupiterSwapApi, SwapMode},
    math::constants::PRICE_PRECISION_U64,
};
use solana_sdk::{
    signature::Signature,
};

use crate::display;

/// Buy JLP using Jupiter swap
pub async fn buy_jlp_via_jupiter(
    client: &DriftClient,
    amount_usdc: f64,
) -> Result<Signature, Box<dyn std::error::Error>> {
    display::print_info(&format!("🔄 Initiating Jupiter swap to buy JLP with {} USDC", amount_usdc));

    let wallet = client.wallet();
    let user_account_pubkey = wallet.default_sub_account();

    // Get user account
    let user: User = client
        .get_user_account(&user_account_pubkey)
        .await?;

    // USDC is spot market 0, JLP is typically market 7 or 19
    let token_in = MarketId::spot(0); // USDC
    let token_out = client.market_lookup("jlp")
        .or_else(|| client.market_lookup("JLP"))
        .unwrap_or_else(|| MarketId::spot(7)); // Try common JLP indices

    display::print_info(&format!("📍 Swapping from USDC (market {}) to JLP (market {})",
        token_in.index(), token_out.index()));

    // Convert USDC amount to base units (6 decimals)
    let amount_in = (amount_usdc * 1_000_000.0) as u64;

    // Query Jupiter for swap route
    display::print_info("🔍 Querying Jupiter for best swap route...");
    let jupiter_swap_info = client
        .jupiter_swap_query(
            wallet.authority(),
            amount_in,
            SwapMode::ExactIn,
            50, // 0.5% slippage
            token_in.index(),
            token_out.index(),
            Some(true), // only direct routes
            None,
            None,
        )
        .await
        .map_err(|e| format!("Failed to query Jupiter swap: {:?}", e))?;

    // Count total instructions (setup + swap + cleanup)
    let total_ixs = jupiter_swap_info.ixs.setup_instructions.len() + 1 +
        jupiter_swap_info.ixs.cleanup_instruction.as_ref().map_or(0, |_| 1);

    display::print_success(&format!("✅ Found swap route with {} instructions", total_ixs));

    // Get token accounts
    let in_market = client
        .program_data()
        .spot_market_config_by_index(token_in.index())
        .ok_or("USDC market not found")?;
    let out_market = client
        .program_data()
        .spot_market_config_by_index(token_out.index())
        .ok_or("JLP market not found")?;

    let in_token_account = Wallet::derive_associated_token_address(&wallet.authority(), &in_market);
    let out_token_account = Wallet::derive_associated_token_address(&wallet.authority(), &out_market);

    // Build transaction
    display::print_info("🔨 Building swap transaction...");
    let tx = TransactionBuilder::new(
        client.program_data(),
        wallet.default_sub_account(),
        std::borrow::Cow::Borrowed(&user),
        false,
    )
    .jupiter_swap(
        jupiter_swap_info,
        &in_market,
        &out_market,
        &in_token_account,
        &out_token_account,
        None,
        None,
    )
    .build();

    // Send transaction
    display::print_info("📤 Sending transaction...");
    let signature = client.sign_and_send(tx).await?;
    display::print_success(&format!("✅ Transaction sent: {}", signature));

    Ok(signature)
}

/// Buy BTC perpetual with market order
pub async fn buy_btc_perp(
    client: &DriftClient,
    amount_usdc: f64,
) -> Result<Signature, Box<dyn std::error::Error>> {
    display::print_info(&format!("📈 Placing BTC-PERP market buy order for {} USDC", amount_usdc));

    let wallet = client.wallet();
    let user_account_pubkey = wallet.default_sub_account();

    // Get user account (not used in this function but might be needed later)
    let _user: User = client
        .get_user_account(&user_account_pubkey)
        .await?;

    // Get BTC-PERP market
    let btc_perp = client
        .market_lookup("btc-perp")
        .ok_or("BTC-PERP market not found")?;

    // Get current BTC price to estimate position size
    let oracle = client.get_oracle_price_data_and_slot(btc_perp).await?;
    let btc_price = oracle.data.price as f64 / PRICE_PRECISION_U64 as f64;
    display::print_info(&format!("📊 Current BTC price: ${:.2}", btc_price));

    // Get market info to check minimum order size
    let market_account = client
        .try_get_perp_market_account(btc_perp.index())
        .map_err(|e| format!("Failed to get BTC-PERP market account: {:?}", e))?;

    let min_order_size = market_account.amm.order_step_size;
    display::print_info(&format!("📏 Market minimum order size: {} base units", min_order_size));

    // Calculate base amount (BTC amount in base units)
    // amount_usdc / btc_price gives us BTC amount
    // Multiply by 10^9 for base precision
    let btc_amount = amount_usdc / btc_price;
    let mut base_amount = (btc_amount * 1_000_000_000.0) as u64;

    // Ensure the order meets minimum size requirements
    if base_amount < min_order_size {
        base_amount = min_order_size;
        let adjusted_btc_amount = base_amount as f64 / 1_000_000_000.0;
        let adjusted_usdc_amount = adjusted_btc_amount * btc_price;
        display::print_info(&format!("⚠️ Order too small, adjusting to minimum: {:.6} BTC (~${:.2} USDC)",
            adjusted_btc_amount, adjusted_usdc_amount));
    } else {
        display::print_info(&format!("📐 Buying approximately {:.6} BTC", btc_amount));
    }

    // Create market order
    let order = NewOrder::market(btc_perp)
        .amount(base_amount as i64) // positive for long
        .build();

    // Build transaction
    display::print_info("🔨 Building order transaction...");
    let tx = client
        .init_tx(&user_account_pubkey, false)
        .await?
        .place_orders(vec![order])
        .build();

    // Send transaction
    display::print_info("📤 Sending transaction...");
    let signature = client.sign_and_send(tx).await?;
    display::print_success(&format!("✅ Transaction sent: {}", signature));

    Ok(signature)
}

/// Monitor transaction status until confirmed
pub async fn monitor_transaction(
    _client: &DriftClient,
    signature: &Signature,
    timeout_secs: u64,
    rpc_url: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    display::print_info(&format!("⏳ Monitoring transaction: {}", signature));

    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    // Create RPC client
    let rpc = drift_rs::event_subscriber::RpcClient::new(rpc_url.to_string());

    loop {
        if start_time.elapsed() > timeout {
            display::print_error(&format!("❌ Transaction timeout after {} seconds", timeout_secs));
            return Ok(false);
        }

        // Get transaction status
        let status = rpc
            .get_signature_statuses(&[*signature])
            .await?;

        if let Some(Some(status)) = status.value.first() {
            if let Some(err) = &status.err {
                display::print_error(&format!("❌ Transaction failed: {:?}", err));
                return Ok(false);
            }

            let confirmations = status.confirmations.unwrap_or(0);

            // Check confirmation status
            if let Some(confirmation_status) = &status.confirmation_status {
                display::print_info(&format!("📍 Status: {:?} ({} confirmations)",
                    confirmation_status, confirmations));

                // Check if transaction is confirmed using Debug format comparison
                let status_str = format!("{:?}", confirmation_status);
                if status_str.contains("Confirmed") || status_str.contains("Finalized") {
                    display::print_success(&format!("✅ Transaction {:?} with {} confirmations",
                        confirmation_status, confirmations));
                    return Ok(true);
                }
            } else {
                display::print_info(&format!("📍 Status: Processing ({} confirmations)", confirmations));
            }
        }

        // Wait before next check
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}