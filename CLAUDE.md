# drift-rs AI Assistant Guide

## Project Overview

**drift-rs** is a high-performance Rust SDK for building offchain clients for Drift V2 protocol on Solana blockchain. This is an experimental SDK focused on performance and reliability for DEX trading operations.

## Key Information

- **Main Language**: Rust
- **Platform**: Solana blockchain
- **Protocol**: Drift V2 DEX protocol
- **Architecture**: x86_64 required (BPF compatibility)
- **License**: Apache-2.0

## Project Structure

```
drift-rs/
├── crates/
│   ├── src/              # Main library source code
│   │   ├── lib.rs         # Entry point
│   │   ├── dlob/          # Decentralized limit order book
│   │   ├── math/          # Mathematical operations
│   │   ├── grpc/          # gRPC client implementation
│   │   └── ...            # Various modules for client functionality
│   ├── drift-ffi-sys/    # FFI bindings to drift program
│   ├── drift-idl-gen/    # IDL type generation
│   └── pubsub-client/    # WebSocket pubsub client
├── examples/             # Example implementations
│   ├── market-maker/
│   ├── swift-maker/
│   ├── swift-taker/
│   └── ...
├── tests/                # Integration tests
│   ├── integration.rs
│   └── jupiter.rs
└── res/                  # Resources (IDL files)
```

## Key Components

### Main Crates

1. **drift-rs** (main crate): Core SDK functionality
   - DriftClient for interacting with Drift protocol
   - WebSocket and gRPC subscription models
   - Account management and caching
   - Transaction crafting

2. **drift-ffi-sys**: FFI bindings to the Drift program
   - Links to protocol-v2 via FFI
   - Version: 2.137.0 (synced with protocol)

3. **drift-pubsub-client**: Patched Solana pubsub client
   - Improved connection handling
   - WebSocket subscriptions

4. **drift-idl-gen**: IDL type generation
   - Generates Rust types from Drift IDL

## Development Setup

### Required Toolchain

**IMPORTANT**: Must use x86_64 architecture due to BPF memory layout compatibility

#### macOS (M-series)
```bash
# Install Rosetta
softwareupdate --install-rosetta

# Install required toolchains
rustup install 1.85.0-x86_64-apple-darwin 1.76.0-x86_64-apple-darwin --force-non-host
rustup override set 1.85.0-x86_64-apple-darwin
```

#### Linux
```bash
rustup install 1.85.0-x86_64-unknown-linux-gnu 1.76.0-x86_64-unknown-linux-gnu --force-non-host
rustup override set 1.85.0-x86_64-unknown-linux-gnu
```

### Build Configuration

Two build modes available:

1. **Build from source** (default):
   ```bash
   export CARGO_DRIFT_FFI_STATIC=1
   cargo build
   ```

2. **Use prebuilt library**:
   ```bash
   export CARGO_DRIFT_FFI_PATH="/path/to/libdrift_ffi_sys"
   cargo build
   ```

## Development Commands

### Essential Commands

```bash
# Format code
cargo fmt --all

# Check compilation
cargo check

# Run clippy linter
cargo clippy --all-targets

# Run tests
cargo test --no-fail-fast --lib -- --nocapture
cargo test --no-fail-fast --test integration -- --nocapture --test-threads 2
cargo test --no-fail-fast --test jupiter -- --nocapture --test-threads 2

# Update IDL types
./scripts/idl-update.sh
cargo check  # Rebuild IDL types

# Build release
cargo build --release
```

### Testing Requirements

Tests require environment variables:
- `TEST_DEVNET_RPC_ENDPOINT`: Devnet RPC endpoint
- `TEST_MAINNET_RPC_ENDPOINT`: Mainnet RPC endpoint
- `TEST_PRIVATE_KEY`: Test wallet private key
- `TEST_MAINNET_PRIVATE_KEY`: Mainnet test wallet
- `TEST_GRPC_X_TOKEN`: gRPC authentication token

## Git Workflow

### Branches
- **main/master**: Primary development branch
- Feature branches should PR to main

### Remote Configuration
- **origin**: https://github.com/drift-labs/drift-rs.git
- **fork**: https://github.com/boyi/drift-rs.git (user's fork)

### Common Git Operations

```bash
# Push to your fork
git push fork main

# Create PR from fork
gh pr create --base drift-labs:main --head boyi:main

# Sync fork with upstream
git fetch origin
git merge origin/main
git push fork main
```

## API Usage Pattern

The SDK follows a subscription model:

1. Create DriftClient with RPC connection
2. Subscribe to required data feeds (markets, oracles, accounts)
3. Access cached data through client methods
4. Build and send transactions

Example:
```rust
let client = DriftClient::new(
    Context::MainNet,
    RpcClient::new("https://rpc.example.com"),
    wallet,
).await?;

// Subscribe to data
client.subscribe_markets(&[MarketId::perp(0)]).await?;
client.subscribe_account("USER_ACCOUNT").await?;

// Access cached data
let price = client.oracle_price(MarketId::perp(0));
```

## Key Features

- **High Performance**: Optimized for low-latency trading
- **Dual Subscription**: WebSocket or gRPC data feeds
- **Transparent Caching**: Live updates cached automatically
- **Type Safety**: Strong typing via generated IDL types
- **Example Implementations**: Market makers, swift trading
- **Flexible gRPC**: Support for HTTP endpoints and optional authentication

## Important Notes

1. **Architecture Requirement**: MUST use x86_64 toolchain. ARM/aarch64 will cause runtime deserialization errors
2. **Submodules**: Remember to init/update git submodules when cloning
3. **FFI Dependency**: Requires libdrift_ffi_sys (build from source or download prebuilt)
4. **Version Compatibility**: drift-ffi-sys version must match protocol-v2 tag

## CI/CD

GitHub Actions workflows:
- **build.yml**: Format, build, test on push/PR
- **release.yml**: Automated releases on tag push
- **on-program-update.yml**: Updates when protocol changes
- **on-libdrift-update.yml**: Updates FFI library

## Recent Updates

### gRPC Enhancements
- **Optional Authentication**: gRPC client now supports connecting without X_TOKEN
- **HTTP Support**: Automatic detection of HTTP vs HTTPS endpoints
- **TLS Control**: Manual TLS configuration via `GrpcConnectionOpts.use_tls`

#### Usage Examples:

```rust
// Connect without authentication
client.grpc_subscribe_with_optional_token(
    "http://localhost:8080".to_string(),
    None, // No X_TOKEN
    opts,
    false
).await?;

// Force HTTP (no TLS)
let opts = GrpcSubscribeOpts::default()
    .connection_opts(GrpcConnectionOpts {
        use_tls: Some(false),
        ..Default::default()
    });

// Auto-detect TLS from URL scheme
let client = DriftGrpcClient::new_with_optional_token(
    "https://grpc.example.com".to_string(), // Auto-enables TLS
    Some("token".to_string())
);
```

## Debugging Tips

1. Deserialization errors like "InvalidSize" usually indicate wrong architecture
2. Check `CARGO_DRIFT_FFI_PATH` or `CARGO_DRIFT_FFI_STATIC` for build issues
3. Integration tests need proper RPC endpoints in environment
4. Use `--nocapture` flag to see test output
5. Run tests with `--test-threads 2` to avoid rate limiting
6. gRPC connection issues: Check if endpoint uses HTTP/HTTPS correctly
7. Authentication errors: Verify if X_TOKEN is required by your gRPC server

## Contact & Support

- Discord: https://discord.com/channels/849494028176588802/878700556904980500
- Documentation: https://docs.drift.trade/developer-resources/sdk-documentation
- Crates.io: https://crates.io/crates/drift-rs