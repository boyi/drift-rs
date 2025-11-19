use std::time::{SystemTime, UNIX_EPOCH};

use colored::*;

/// Format a price with proper precision and color
pub fn format_price(price: u64, precision: u64) -> String {
    let formatted = format!("{:.6}", price as f64 / precision as f64);
    formatted.bright_cyan().to_string()
}

/// Format a percentage change with color coding
pub fn format_percentage_change(change: f64) -> String {
    let formatted = format!("{:+.2}%", change * 100.0);
    if change > 0.0 {
        formatted.bright_green().to_string()
    } else if change < 0.0 {
        formatted.bright_red().to_string()
    } else {
        formatted.white().to_string()
    }
}

/// Format a balance amount with currency symbol
pub fn format_balance(amount: i64, precision: u64, symbol: &str) -> String {
    let balance = amount as f64 / precision as f64;
    if balance >= 0.0 {
        format!("{:.6} {}", balance, symbol).bright_green().to_string()
    } else {
        format!("{:.6} {}", balance, symbol).bright_red().to_string()
    }
}

/// Format unrealized PnL with color coding
pub fn format_pnl(pnl: i64, precision: u64) -> String {
    let pnl_value = pnl as f64 / precision as f64;
    let formatted = format!("${:.2}", pnl_value);
    if pnl > 0 {
        formatted.bright_green().to_string()
    } else if pnl < 0 {
        formatted.bright_red().to_string()
    } else {
        formatted.white().to_string()
    }
}

/// Get current timestamp string
pub fn current_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Format as HH:MM:SS
    let hours = (now % 86400) / 3600;
    let minutes = (now % 3600) / 60;
    let seconds = now % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds).bright_black().to_string()
}

/// Print a header with decorative borders
pub fn print_header(title: &str) {
    println!("{}", "═".repeat(60).bright_blue());
    println!("{}", format!("  🚀 {}", title).bright_white().bold());
    println!("{}", "═".repeat(60).bright_blue());
}

/// Print a section divider
pub fn print_divider() {
    println!("{}", "─".repeat(60).bright_black());
}

/// Print price update notification
pub fn print_price_update(market: &str, old_price: f64, new_price: f64, precision: u64) {
    let change = (new_price - old_price) / old_price;
    let timestamp = current_timestamp();

    println!(
        "{} {} Price: {} {} {}",
        timestamp,
        "📈".bright_yellow(),
        market.bright_white().bold(),
        format_price(new_price as u64, precision),
        format_percentage_change(change)
    );
}

/// Print balance update notification
pub fn print_balance_update(asset: &str, old_balance: i64, new_balance: i64, precision: u64) {
    let change = new_balance - old_balance;
    let timestamp = current_timestamp();

    let change_str = if change > 0 {
        format!("+{:.6}", change as f64 / precision as f64).bright_green()
    } else if change < 0 {
        format!("{:.6}", change as f64 / precision as f64).bright_red()
    } else {
        format!("0.000000").white()
    };

    println!(
        "{} {} Balance: {} {} ({})",
        timestamp,
        "💰".bright_yellow(),
        asset.bright_white().bold(),
        format_balance(new_balance, precision, ""),
        change_str
    );
}

/// Print position update notification
pub fn print_position_update(market: &str, size: i64, pnl: i64, price_precision: u64, base_precision: u64) {
    let timestamp = current_timestamp();

    println!(
        "{} {} Position: {} size: {:.6} PnL: {}",
        timestamp,
        "📊".bright_yellow(),
        market.bright_white().bold(),
        size as f64 / base_precision as f64,
        format_pnl(pnl, price_precision)
    );
}

/// Print status summary
pub fn print_status_summary(
    btc_perp_price: Option<f64>,
    usdc_balance: Option<i64>,
    jlp_balance: Option<i64>,
    btc_position_size: Option<i64>,
    btc_position_pnl: Option<i128>,
    btc_funding_rate: Option<i64>,
    btc_funding_rate_24h: Option<i64>,
    btc_oracle_twap: Option<i64>,
    sol_funding_rate: Option<i64>,
    sol_funding_rate_24h: Option<i64>,
    sol_oracle_twap: Option<i64>,
    eth_funding_rate: Option<i64>,
    eth_funding_rate_24h: Option<i64>,
    eth_oracle_twap: Option<i64>,
) {
    print_divider();
    println!("{}", "📋 Current Status".bright_white().bold());

    if let Some(price) = btc_perp_price {
        println!("  BTC-PERP: {}", format_price(price as u64, 1_000_000));
    } else {
        println!("  BTC-PERP: {}", "No price data".bright_black());
    }

    if let Some(balance) = usdc_balance {
        println!("  USDC Balance: {}", format_balance(balance, 1_000_000, "USDC"));
    } else {
        println!("  USDC Balance: {}", "No account data".bright_black());
    }

    if let Some(balance) = jlp_balance {
        println!("  JLP Balance: {}", format_balance(balance, 1_000_000, "JLP"));
    } else {
        println!("  JLP Balance: {}", "No JLP position".bright_black());
    }

    if let (Some(size), Some(pnl)) = (btc_position_size, btc_position_pnl) {
        println!(
            "  BTC Position: {:.6} (PnL: {})",
            size as f64 / 1_000_000_000.0,
            format_pnl(pnl as i64, 1_000_000)
        );
    } else {
        println!("  BTC Position: {}", "No position data".bright_black());
    }

    if let (Some(funding_rate), Some(funding_rate_24h), Some(oracle_twap)) = (btc_funding_rate, btc_funding_rate_24h, btc_oracle_twap) {
        // Formula: (last_funding_rate / last_funding_oracle_twap) / FUNDING_RATE_BUFFER * 100 (for percentage)
        // FUNDING_RATE_BUFFER = 1000, so: / 1000 * 100 = / 10
        let rate_pct = funding_rate as f64 / oracle_twap as f64 / 10.0;
        let rate_24h_pct = funding_rate_24h as f64 / oracle_twap as f64 / 10.0;

        let rate_str = if rate_pct > 0.0 {
            format!("{:+.6}%", rate_pct).bright_green()
        } else if rate_pct < 0.0 {
            format!("{:+.6}%", rate_pct).bright_red()
        } else {
            format!("{:+.6}%", rate_pct).white()
        };

        let rate_24h_str = if rate_24h_pct > 0.0 {
            format!("{:+.6}%", rate_24h_pct).bright_green()
        } else if rate_24h_pct < 0.0 {
            format!("{:+.6}%", rate_24h_pct).bright_red()
        } else {
            format!("{:+.6}%", rate_24h_pct).white()
        };

        println!("  BTC Funding Rate: {} (24h avg: {})", rate_str, rate_24h_str);
    } else {
        println!("  BTC Funding Rate: {}", "No funding rate data".bright_black());
    }

    if let (Some(funding_rate), Some(funding_rate_24h), Some(oracle_twap)) = (sol_funding_rate, sol_funding_rate_24h, sol_oracle_twap) {
        // Formula: (last_funding_rate / last_funding_oracle_twap) / FUNDING_RATE_BUFFER * 100 (for percentage)
        // FUNDING_RATE_BUFFER = 1000, so: / 1000 * 100 = / 10
        let rate_pct = funding_rate as f64 / oracle_twap as f64 / 10.0;
        let rate_24h_pct = funding_rate_24h as f64 / oracle_twap as f64 / 10.0;

        let rate_str = if rate_pct > 0.0 {
            format!("{:+.6}%", rate_pct).bright_green()
        } else if rate_pct < 0.0 {
            format!("{:+.6}%", rate_pct).bright_red()
        } else {
            format!("{:+.6}%", rate_pct).white()
        };

        let rate_24h_str = if rate_24h_pct > 0.0 {
            format!("{:+.6}%", rate_24h_pct).bright_green()
        } else if rate_24h_pct < 0.0 {
            format!("{:+.6}%", rate_24h_pct).bright_red()
        } else {
            format!("{:+.6}%", rate_24h_pct).white()
        };

        println!("  SOL Funding Rate: {} (24h avg: {})", rate_str, rate_24h_str);
    } else {
        println!("  SOL Funding Rate: {}", "No funding rate data".bright_black());
    }

    if let (Some(funding_rate), Some(funding_rate_24h), Some(oracle_twap)) = (eth_funding_rate, eth_funding_rate_24h, eth_oracle_twap) {
        // Formula: (last_funding_rate / last_funding_oracle_twap) / FUNDING_RATE_BUFFER * 100 (for percentage)
        // FUNDING_RATE_BUFFER = 1000, so: / 1000 * 100 = / 10
        let rate_pct = funding_rate as f64 / oracle_twap as f64 / 10.0;
        let rate_24h_pct = funding_rate_24h as f64 / oracle_twap as f64 / 10.0;

        let rate_str = if rate_pct > 0.0 {
            format!("{:+.6}%", rate_pct).bright_green()
        } else if rate_pct < 0.0 {
            format!("{:+.6}%", rate_pct).bright_red()
        } else {
            format!("{:+.6}%", rate_pct).white()
        };

        let rate_24h_str = if rate_24h_pct > 0.0 {
            format!("{:+.6}%", rate_24h_pct).bright_green()
        } else if rate_24h_pct < 0.0 {
            format!("{:+.6}%", rate_24h_pct).bright_red()
        } else {
            format!("{:+.6}%", rate_24h_pct).white()
        };

        println!("  ETH Funding Rate: {} (24h avg: {})", rate_str, rate_24h_str);
    } else {
        println!("  ETH Funding Rate: {}", "No funding rate data".bright_black());
    }

    print_divider();
}

/// Print error message
pub fn print_error(message: &str) {
    println!("{} {}", "❌".bright_red(), message.bright_red());
}

/// Print success message
pub fn print_success(message: &str) {
    println!("{} {}", "✅".bright_green(), message.bright_green());
}

/// Print funding rate update notification
pub fn print_funding_rate_update(market: &str, funding_rate: i64, funding_rate_24h: i64, oracle_twap: i64) {
    let timestamp = current_timestamp();

    // Formula: (last_funding_rate / last_funding_oracle_twap) / FUNDING_RATE_BUFFER * 100 (for percentage)
    // FUNDING_RATE_BUFFER = 1000, so: / 1000 * 100 = / 10
    let rate_pct = funding_rate as f64 / oracle_twap as f64 / 10.0;
    let rate_24h_pct = funding_rate_24h as f64 / oracle_twap as f64 / 10.0;

    let rate_str = if rate_pct > 0.0 {
        format!("{:+.6}%", rate_pct).bright_green()
    } else if rate_pct < 0.0 {
        format!("{:+.6}%", rate_pct).bright_red()
    } else {
        format!("{:+.6}%", rate_pct).white()
    };

    let rate_24h_str = if rate_24h_pct > 0.0 {
        format!("{:+.6}%", rate_24h_pct).bright_green()
    } else if rate_24h_pct < 0.0 {
        format!("{:+.6}%", rate_24h_pct).bright_red()
    } else {
        format!("{:+.6}%", rate_24h_pct).white()
    };

    println!(
        "{} {} Funding Rate: {} current: {} 24h avg: {}",
        timestamp,
        "💰".bright_yellow(),
        market.bright_white().bold(),
        rate_str,
        rate_24h_str
    );
}

/// Print info message
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ️ ".bright_blue(), message.bright_white());
}