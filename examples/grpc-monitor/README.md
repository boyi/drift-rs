# gRPC Real-time Monitor

Real-time monitoring of BTC prices and account balances using Drift Protocol's gRPC streaming API.

## Features

- 🚀 **Real-time Price Updates**: Monitor BTC-PERP price changes with customizable thresholds
- 💰 **Balance Monitoring**: Track USDC spot balance changes in real-time
- 📊 **Position Tracking**: Monitor BTC perpetual positions and unrealized PnL
- 🎨 **Colorful Output**: Color-coded display for easy reading
- 🔐 **Flexible Authentication**: Support for optional authentication (great for local testing)
- 🌐 **HTTP/HTTPS Support**: Auto-detect TLS based on URL or manually configure
- 📁 **Secure Key Management**: Load private keys from JSON files

## Usage

### Setup

1. **Configure Environment Variables** (recommended):
   ```bash
   cp .env.example .env
   # Edit .env with your gRPC endpoint and credentials
   ```

2. **Basic Usage**:
   ```bash
   # Monitor with a Solana CLI wallet file (reads URLs from .env)
   cargo run -- --wallet-file ~/.config/solana/id.json

   # Use mainnet instead of devnet
   cargo run -- --wallet-file wallet.json --mainnet
   ```

### Advanced Configuration

```bash
# Override gRPC endpoint from command line
cargo run -- \
  --wallet-file wallet.json \
  --grpc-url http://localhost:10000 \
  --no-auth

# Set price change threshold to 0.1% (if not set, shows all price changes)
cargo run -- \
  --wallet-file wallet.json \
  --price-threshold 0.001

# Show all price changes (no threshold filtering)
cargo run -- \
  --wallet-file wallet.json

# Override both RPC and gRPC endpoints
cargo run -- \
  --wallet-file wallet.json \
  --rpc-url https://api.mainnet-beta.solana.com \
  --grpc-url https://your-grpc-provider.com \
  --grpc-token YOUR_TOKEN_HERE
```

## Wallet File Formats

### Solana CLI Format (Recommended)
```json
[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64]
```

### Custom Format
```json
{
  "private_key": "5J8QhkrGf8mGPiVhjx4jQZKkFyDV3rNQpPEV8g8z8XcVvTvYhUg..."
}
```

## Environment Variables

The application reads configuration from environment variables (`.env` file):

| Variable | Description | Required | Example |
|----------|-------------|----------|---------|
| `GRPC_URL` | gRPC endpoint URL | **Yes** | `http://localhost:10000` |
| `GRPC_X_TOKEN` | Authentication token | No | `your-auth-token` |
| `RPC_URL` | Solana RPC endpoint | No | `https://api.devnet.solana.com` |

**Important**: Standard Solana RPC endpoints (like `api.mainnet-beta.solana.com`) do NOT support gRPC. You need a specialized gRPC provider like Yellowstone gRPC.

## Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--wallet-file, -w` | Path to wallet JSON file | *Required* |
| `--grpc-url` | gRPC endpoint URL | From `GRPC_URL` env var |
| `--grpc-token` | Authentication token | From `GRPC_X_TOKEN` env var |
| `--no-auth` | Disable authentication | false |
| `--rpc-url` | Solana RPC endpoint | From `RPC_URL` env var or network default |
| `--price-threshold` | Price change alert threshold | None (shows all changes) |
| `--mainnet` | Use mainnet instead of devnet | false |

## What You'll See

The monitor displays real-time updates with color coding:

```
🚀 BTC-USDC gRPC Monitor
═══════════════════════════════════════════════════════════

📁 Loading wallet from: wallet.json
💰 Wallet address: 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU
🌐 Using MainNet context
🔓 No authentication (useful for local testing)
🔗 gRPC endpoint: http://localhost:8080
📊 Price change threshold: 0.5%

✅ Drift client initialized
✅ gRPC subscription active
✅ Starting real-time monitoring...

12:34:56 📈 BTC-PERP Price: 67,234.500000 +0.15%
12:35:12 💰 USDC Balance: 1,234.567890 (+50.000000)
12:35:28 📊 BTC-PERP Position: size: 0.100000 PnL: $12.34

─────────────────────────────────────────────────────────────
📋 Current Status
  BTC-PERP: 67,234.500000
  USDC Balance: 1,234.567890 USDC
  BTC Position: 0.100000 (PnL: $12.34)
─────────────────────────────────────────────────────────────
```

## Monitoring Behavior

- **Price Updates**: Displayed when BTC-PERP price changes (all changes if no threshold set)
- **Balance Updates**: Shown whenever USDC spot balance changes
- **Position Updates**: Displayed when position size or PnL changes
- **Status Summary**: Printed every 30 seconds regardless of changes
- **Color Coding**:
  - 🟢 Green: Positive values, gains
  - 🔴 Red: Negative values, losses
  - 🔵 Blue: Neutral information
  - ⚪ White: Labels and text

## Local Development & Testing

For local testing with a development gRPC server:

```bash
# Run against local HTTP gRPC server without authentication
cargo run -- \
  --wallet-file test-wallet.json \
  --grpc-url http://localhost:8080 \
  --no-auth \
  --price-threshold 0.001
```

## Troubleshooting

### Common Issues

1. **Wallet file not found**:
   ```
   ❌ Failed to read wallet file 'wallet.json': No such file or directory
   ```
   Make sure the wallet file path is correct and the file exists.

2. **Invalid wallet format**:
   ```
   ❌ Unsupported wallet file format. Expected Solana CLI format [1,2,3,...] or {"private_key": "base58..."}
   ```
   Check that your wallet file is in one of the supported formats.

3. **gRPC connection failed**:
   ```
   ❌ gRPC subscription failed: connection refused
   ```
   - Check that the gRPC endpoint is correct and reachable
   - For HTTPS endpoints, ensure TLS is working
   - For HTTP endpoints, make sure the server supports HTTP/2

4. **Network mismatch error**:
   ```
   thread 'main' panicked at crates/src/lib.rs:1119:71: called `Option::unwrap()` on a `None` value
   ```
   - This usually means network context doesn't match RPC endpoints
   - If using `--mainnet`, ensure RPC and gRPC URLs point to mainnet
   - If using devnet (default), ensure endpoints point to devnet
   - Example fix: `cargo run -- --wallet-file wallet.json --mainnet --rpc-url https://api.mainnet-beta.solana.com`

5. **Authentication errors**:
   ```
   ❌ gRPC subscription failed: authentication failed
   ```
   - Check that your `GRPC_X_TOKEN` is valid
   - Try using `--no-auth` for local testing
   - Verify the token has necessary permissions

## Dependencies

- `drift-rs`: Drift Protocol Rust SDK
- `argh`: Command line argument parsing
- `colored`: Terminal color output
- `serde`: JSON serialization
- `tokio`: Async runtime

## License

This example follows the same license as the drift-rs project.