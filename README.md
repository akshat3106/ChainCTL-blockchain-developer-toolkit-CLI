# ChainCTL

**A blockchain developer toolkit CLI** — testnet faucet discovery, RPC diagnostics, gas/wallet/tx lookups, ABI encoding, contract reads, and ENS resolution, all in one command-line tool that feels like `git`/`kubectl`/`cast`.

```
$ chainctl faucet recommend base --explain
Recommended: Base Official Faucet for Base Sepolia — score 81.7/100
  https://portal.cdp.coinbase.com/products/faucet

  Breakdown:
    availability     +30.0
    community        +9.1
    latency          +7.9
    official         +40.0
    recentFailures   +0.0
```

No API keys required for anything in this README. Every example below was run against real, live endpoints.

---

## Table of Contents

- [What is ChainCTL?](#what-is-chainctl)
- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Global Flags](#global-flags)
- [Command Reference](#command-reference)
  - [`chains`](#chains)
  - [`faucet`](#faucet)
  - [`rpc`](#rpc)
  - [`network`](#network)
  - [`explorer`](#explorer)
  - [`gas`](#gas)
  - [`wallet`](#wallet)
  - [`tx`](#tx)
  - [`abi`](#abi)
  - [`contract`](#contract)
  - [`ens`](#ens)
  - [`update`, `cache`, `config`, `doctor`, `version`](#update-cache-config-doctor-version)
  - [Plugins](#plugins)
- [Configuration](#configuration)
- [Supported Chains](#supported-chains)
- [Exit Codes & Errors](#exit-codes--errors)
- [Architecture](#architecture)
- [License](#license)

---

## What is ChainCTL?

Getting testnet tokens, checking an RPC endpoint, or reading a contract value shouldn't require five browser tabs, a Discord search, and a half-remembered curl command. ChainCTL puts all of it behind one CLI:

- **Discover and rank testnet faucets** across 7 chains, backed by a live weighted-scoring engine (not just a static link list).
- **Diagnose RPC endpoints** — reachability, latency, and whether they're actually serving the chain they claim to.
- **Read gas prices, wallet balances, and transaction status** without leaving the terminal.
- **Encode/decode ABI calldata and call read-only contract functions** with `cast`-style human-readable signatures — no API key, no ABI JSON file required.
- **Resolve ENS names** against real Ethereum mainnet.
- **Add your own custom chains** on top of the built-in registry.

Everything is a single static binary. No Node, no Python, no config required to get started.

## Features

| Module | What it does |
|---|---|
| `chains` | List/inspect the built-in + your custom chains |
| `faucet` | Search, rank, open, and health-check testnet faucets |
| `rpc` | Test RPC endpoint reachability/correctness, benchmark latency |
| `network` | Add your own chains on top of the registry |
| `explorer` | Jump straight to a block explorer (homepage, tx, or address) |
| `gas` | Current gas price, gas estimation |
| `wallet` | Read-only balance lookups (no keys, ever) |
| `tx` | Transaction status by hash |
| `abi` | Encode/decode function calldata |
| `contract` | Call read-only contract functions |
| `ens` | Resolve `name.eth` ⇄ address |

## Installation

No Rust toolchain required — prebuilt binaries are published for macOS, Linux, and Windows.

### One-command install (recommended)

**macOS / Linux:**

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/akshat3106/ChainCTL-blockchain-developer-toolkit-CLI/releases/latest/download/chainctl-installer.sh | sh
```

**Windows (PowerShell):**

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/akshat3106/ChainCTL-blockchain-developer-toolkit-CLI/releases/latest/download/chainctl-installer.ps1 | iex"
```

The installer downloads the right binary for your platform, places it in your Cargo home
(`~/.cargo/bin`, or `%USERPROFILE%\.cargo\bin` on Windows), and adds that directory to your
`PATH` if it isn't there already.

> **Open a new terminal afterwards.** A shell that was already running keeps its own copy of
> `PATH` from when it started, so `chainctl` won't be found in that window until you restart it.
> This is not a bug — it's how `PATH` works on every OS.

### Manual install

Prefer to do it by hand, or the installer didn't fit your setup? Grab an archive from the
[latest release](https://github.com/akshat3106/ChainCTL-blockchain-developer-toolkit-CLI/releases/latest):

| Platform | Archive |
|---|---|
| Apple Silicon macOS | `chainctl-aarch64-apple-darwin.tar.xz` |
| Intel macOS | `chainctl-x86_64-apple-darwin.tar.xz` |
| x64 Linux | `chainctl-x86_64-unknown-linux-gnu.tar.xz` |
| ARM64 Linux | `chainctl-aarch64-unknown-linux-gnu.tar.xz` |
| x64 Windows | `chainctl-x86_64-pc-windows-msvc.zip` |

**macOS / Linux** — extract and put the binary on your `PATH`:

```bash
tar -xf chainctl-x86_64-unknown-linux-gnu.tar.xz
sudo mv chainctl-x86_64-unknown-linux-gnu/chainctl /usr/local/bin/
chainctl --version
```

**Windows (PowerShell)** — extract, then copy to a folder on your `PATH`:

```powershell
Expand-Archive chainctl-x86_64-pc-windows-msvc.zip -DestinationPath .
New-Item -ItemType Directory -Force "$env:USERPROFILE\bin" | Out-Null
Copy-Item .\chainctl-x86_64-pc-windows-msvc\chainctl.exe "$env:USERPROFILE\bin\chainctl.exe"
```

If `$env:USERPROFILE\bin` isn't on your `PATH` yet, add it once (persists for your user):

```powershell
[Environment]::SetEnvironmentVariable(
    "Path",
    [Environment]::GetEnvironmentVariable("Path", "User") + ";$env:USERPROFILE\bin",
    "User")
```

Then **open a new PowerShell window** and run `chainctl --version`.

### Verify the checksum (optional)

Every archive ships with a matching `.sha256`:

```bash
sha256sum -c chainctl-x86_64-unknown-linux-gnu.tar.xz.sha256
```

```powershell
# Windows: compare against the .sha256 file's contents
Get-FileHash .\chainctl-x86_64-pc-windows-msvc.zip -Algorithm SHA256
```

### Build from source

Requires the Rust toolchain (recent stable). Install it from [rustup.rs](https://rustup.rs) if needed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # macOS/Linux
winget install Rustlang.Rustup                                   # Windows
```

Then:

```bash
git clone https://github.com/akshat3106/ChainCTL-blockchain-developer-toolkit-CLI.git
cd ChainCTL-blockchain-developer-toolkit-CLI
cargo build --release
```

The binary lands at `target/release/chainctl` (`target\release\chainctl.exe` on Windows). Copy it
onto your `PATH` as shown above, or run it in place during development:

```bash
cargo run -- chains
```

### Confirm it works

```bash
$ chainctl doctor
✓ Config directory       /home/you/.chainctl
✓ Embedded registry      7 chains, 9 faucets
✓ Local registry.json    not present yet — run `chainctl update`
```

> No network access needed for this check — the registry ships embedded in the binary.

**If `chainctl` isn't found:** the binary exists but its folder isn't on your `PATH`. Open a new
terminal first (see the note above). If it still isn't found, check the folder is really on `PATH`
(`echo $PATH` / `$env:Path`) and that the binary is really in it. Running it by full path —
`~/.cargo/bin/chainctl --version` — confirms the binary itself is fine and isolates the problem
to `PATH`.

## Quick Start

```bash
# See what chains are supported
chainctl chains

# Find the best faucet for Base Sepolia and open it in your browser
chainctl faucet open base

# Or just see the ranking first
chainctl faucet recommend base --explain

# Check an RPC endpoint
chainctl rpc test base

# Check a wallet balance
chainctl wallet balance base 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

## Global Flags

These work on every subcommand:

| Flag | Description |
|---|---|
| `-o, --output <table\|json\|plain>` | Output format. `json` is stable and scriptable. |
| `--no-color` | Disable ANSI color (also respects the `NO_COLOR` env var). |
| `--config-dir <path>` | Use a different config/cache root instead of `~/.chainctl`. |
| `-q, --quiet` | Suppress non-essential status output. |
| `--fresh` | Bypass any cached data (registry, health results) for this run. |

---

## Command Reference

### `chains`

List every known chain, or inspect one.

```bash
chainctl chains
chainctl chains info base
```

```
$ chainctl chains
┌──────────────────┬──────────────────┬──────────┬────────┬─────────┬─────────┐
│ ID               ┆ Name             ┆ Chain ID ┆ Symbol ┆ Network ┆ Faucets │
╞══════════════════╪══════════════════╪══════════╪════════╪═════════╪═════════╡
│ ethereum-sepolia ┆ Ethereum Sepolia ┆ 11155111 ┆ ETH    ┆ testnet ┆ 2       │
│ base-sepolia     ┆ Base Sepolia     ┆ 84532    ┆ ETH    ┆ testnet ┆ 2       │
│ polygon-amoy     ┆ Polygon Amoy     ┆ 80002    ┆ POL    ┆ testnet ┆ 1       │
│ ...              ┆                  ┆          ┆        ┆         ┆         │
└──────────────────┴──────────────────┴──────────┴────────┴─────────┴─────────┘
```

Chains can be referenced by id or alias — `base`, `eth`/`sepolia`, `polygon`/`amoy`, `op`, `arb`, `avax`/`fuji`, `bnb`/`bsc` all work (see [Supported Chains](#supported-chains)).

### `faucet`

```bash
chainctl faucet search <chain> [--source official|partner|community]
chainctl faucet info <chain> [--faucet <id>]
chainctl faucet open <chain> [--faucet <id>]
chainctl faucet status [chain] [--watch] [--interval <secs>]
chainctl faucet recommend <chain> [--explain]
```

- **`search`** lists every faucet for a chain, optionally filtered by source.
- **`info`** shows full detail (requirements, cooldown, amount per claim) for the top-ranked faucet, or a specific one via `--faucet <id>`.
- **`open`** launches the recommended (or a specific) faucet in your default browser.
- **`status`** health-checks faucets concurrently — real HTTP probes plus TLS certificate-expiry inspection. `--watch` repeats it on an interval (15s floor, to avoid hammering faucet servers).
- **`recommend`** ranks faucets with a weighted scoring engine (official source, live availability/latency from cached health checks, community rating, recent failures). `--explain` shows the per-factor breakdown.

```
$ chainctl faucet status base
┌──────────────┬─────────────────────────────┬────────┬──────┬─────────┬──────┐
│ Chain        ┆ Faucet                      ┆ Status ┆ HTTP ┆ Latency ┆ SSL  │
╞══════════════╪═════════════════════════════╪════════╪══════╪═════════╪══════╡
│ base-sepolia ┆ Alchemy Base Sepolia Faucet ┆ Online ┆ 200  ┆ 195ms   ┆ 62d  │
│ base-sepolia ┆ Base Official Faucet        ┆ Online ┆ 200  ┆ 428ms   ┆ 85d  │
└──────────────┴─────────────────────────────┴────────┴──────┴─────────┴──────┘
```

Health results are cached (`cache.ttlMinutes`, default 30) — `recommend`/`open`/`info` reuse them instead of re-probing every time.

### `rpc`

```bash
chainctl rpc list [chain]
chainctl rpc test [chain]
chainctl rpc latency <chain> [--samples <n>]
```

- **`list`** shows the registry's known RPC URL(s) per chain.
- **`test`** does a real `eth_chainId` JSON-RPC call and checks the response against the registry's expected chain ID — catching a misconfigured endpoint, not just a dead one.
- **`latency`** samples repeated calls and reports min/avg/max.

```
$ chainctl rpc test base
┌──────────────┬───────────────────────────┬───────────┬─────────────┬─────────┬───────┐
│ Chain        ┆ URL                       ┆ Reachable ┆ Chain ID OK ┆ Latency ┆ Error │
╞══════════════╪═══════════════════════════╪═══════════╪═════════════╪═════════╪═══════╡
│ base-sepolia ┆ https://sepolia.base.org  ┆ yes       ┆ yes         ┆ 398ms   ┆       │
└──────────────┴───────────────────────────┴───────────┴─────────────┴─────────┴───────┘
```

### `network`

Add your own chains on top of the built-in registry — useful for a private devnet or a chain that isn't in ChainCTL yet.

```bash
chainctl network add <id> --name <name> --chain-id <n> --symbol <sym> --rpc-url <url> \
  [--explorer-url <url>] [--network testnet|mainnet|devnet] [--parent-chain <id>] [--alias <a>]...
chainctl network list
chainctl network remove <id>
```

```bash
$ chainctl network add my-devnet --name "My Devnet" --chain-id 99999 --symbol DEV --rpc-url https://rpc.example.com
Added custom network 'my-devnet'. Run `chainctl chains info my-devnet` to see it.
```

Custom chains show up everywhere — `chains`, `faucet`, `rpc`, `gas`, `wallet`, `tx`, `contract` — not just in `network list`.

### `explorer`

```bash
chainctl explorer open <chain>
chainctl explorer tx <chain> <hash>
chainctl explorer address <chain> <address>
```

Opens the right block explorer page in your browser.

### `gas`

```bash
chainctl gas price <chain>
chainctl gas estimate <chain> [--to <addr>] [--from <addr>] [--value <wei-or-hex>] [--data <hex>]
```

```
$ chainctl gas price base
Base Sepolia: 0.006 Gwei (6000000 wei)

$ chainctl gas estimate base --to 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045 --value 1000000000000000
Estimated gas: 21000 units
```

`--value` accepts a plain decimal wei amount or a `0x`-prefixed hex quantity.

### `wallet`

Read-only. ChainCTL never generates, stores, or handles private keys — see [Architecture](#architecture) for why.

```bash
chainctl wallet balance <chain> <address>
```

```
$ chainctl wallet balance base 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045: 5.557501 ETH (5557501108355931924 wei)
```

### `tx`

```bash
chainctl tx status <chain> <hash>
```

```
$ chainctl tx status base 0xabc123...
0xabc123...: success
  Block:    12345678
  Gas used: 21000
  From:     0x...
  To:       0x...
```

### `abi`

Encode/decode function calldata using `cast`-style human-readable signatures — no ABI JSON file, no API key.

```bash
chainctl abi encode "<signature>" <args...>
chainctl abi decode "<signature>" <calldata>
```

Supported types: `address`, `bool`, `string`, `bytes`, `bytes1`–`bytes32`, `uint8`–`uint256`, `int8`–`int256` (non-negative only). Arrays and tuples aren't supported yet.

```
$ chainctl abi encode "transfer(address,uint256)" 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045 1000000000000000000
0xa9059cbb000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa960450000000000000000000000000000000000000000000000000de0b6b3a7640000

$ chainctl abi decode "transfer(address,uint256)" 0xa9059cbb0000...
address: 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
uint256: 1000000000000000000
```

(That's the real, standard ERC-20 `transfer` selector — `0xa9059cbb` — computed from scratch via keccak256, not hardcoded.)

### `contract`

Call a read-only contract function. The signature includes both input *and* output types: `"name(inputTypes)(outputTypes)"`.

```bash
chainctl contract read <chain> <address> "<signature>" <args...>
```

```
$ chainctl contract read base 0xcA11bde05977b3631167028862bE2a173976CA11 "getChainId()(uint256)"
uint256: 84532
```

(That's calling the real Multicall3 contract — deployed at the same address on almost every EVM chain — and getting back Base Sepolia's actual chain ID.)

### `ens`

Resolves against real Ethereum mainnet (configurable via `ens.rpcUrl`), independent of whatever testnet the rest of ChainCTL is pointed at.

```bash
chainctl ens resolve <name>
chainctl ens reverse <address>
```

```
$ chainctl ens resolve vitalik.eth
vitalik.eth -> 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045

$ chainctl ens reverse 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045 -> vitalik.eth
```

### `update`, `cache`, `config`, `doctor`, `version`

```bash
chainctl update                      # refresh the local registry (falls back to the bundled snapshot if no source is configured)
chainctl cache clear                 # wipe ~/.chainctl/cache
chainctl cache info                  # cache size/location
chainctl cache refresh               # update + clear, in one step
chainctl config get <key>            # e.g. `chainctl config get recommend.weights.official`
chainctl config set <key> <value>    # e.g. `chainctl config set health.concurrency 10`
chainctl config list                 # print the whole resolved config
chainctl config edit                 # open config.yaml in your default editor
chainctl doctor                      # environment sanity check
chainctl version
```

### Plugins

Any unrecognized subcommand is looked up as `chainctl-<name>` on your `$PATH` and run with the remaining arguments — the same convention `git` and `kubectl` use. Ship your own module without forking ChainCTL:

```bash
# if chainctl-foo is somewhere on $PATH:
chainctl foo bar baz
# -> runs `chainctl-foo bar baz`
```

---

## Configuration

`~/.chainctl/config.yaml` (created on first run). Every key has a sane default — you only need to touch what you want to change.

```yaml
version: 1
output:
  format: table          # table | json | plain
  color: auto
  icons: true
registry:
  source: ""              # URL to fetch registry.json from; empty = use the bundled snapshot
  updateIntervalHours: 24
cache:
  ttlMinutes: 30           # how long faucet health results stay fresh
health:
  concurrency: 5           # max concurrent health/RPC checks
  timeoutSeconds: 5
recommend:
  weights:                 # faucet recommendation scoring — see ARCHITECTURE.md §10
    official: 0.40
    availability: 0.30
    latency: 0.15
    community: 0.10
    recentFailures: 0.05
ens:
  rpcUrl: "https://ethereum-rpc.publicnode.com"   # mainnet RPC used only by `ens`
telemetry:
  enabled: false           # reserved; nothing is ever sent — see ARCHITECTURE.md
```

Other config files in `~/.chainctl/`:

| File | Purpose |
|---|---|
| `registry.json` | Last-fetched registry (via `chainctl update`) |
| `registry.override.json` | Your custom chains (via `chainctl network add`) |
| `cache/health.json` | Cached faucet/RPC health results |

## Supported Chains

| ID | Aliases | Chain ID | Symbol |
|---|---|---|---|
| `ethereum-sepolia` | `ethereum`, `eth`, `sepolia` | 11155111 | ETH |
| `base-sepolia` | `base`, `base-testnet` | 84532 | ETH |
| `polygon-amoy` | `polygon`, `amoy` | 80002 | POL |
| `optimism-sepolia` | `optimism`, `op`, `op-sepolia` | 11155420 | ETH |
| `arbitrum-sepolia` | `arbitrum`, `arb`, `arb-sepolia` | 421614 | ETH |
| `avalanche-fuji` | `avalanche`, `avax`, `fuji` | 43113 | AVAX |
| `bnb-testnet` | `bnb`, `bsc`, `bsc-testnet` | 97 | tBNB |

Add your own with `chainctl network add` (see [`network`](#network)).

## Exit Codes & Errors

Errors print as `✗ <message>` with a `→ <hint>` line telling you what to do next. Exit codes:

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | I/O error |
| `3` | Network/offline/timeout |
| `4` | Bad input — unknown chain, missing data, invalid config |

`--output json` emits `{"error": {"message": ..., "hint": ...}}` instead, for scripting.

## Architecture

ChainCTL is a Cargo workspace (`chainctl-core`, `chainctl-provider`, `chainctl-output`, `chainctl-registry`, `chainctl-scoring`, plus the `chainctl` binary), with the crate dependency graph itself enforcing the layering — a pure domain layer, an I/O adapter layer, and a presentation layer that can't accidentally reach into each other.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full design doc: technology decisions, module responsibilities, the registry JSON schema, the recommendation-scoring algorithm, and the reasoning behind every major tradeoff (why RPC calls get no per-host throttle but faucet checks do, why `wallet` is read-only, why ABI encoding is hand-rolled instead of pulling in `alloy`, etc.).

## License

MIT (see `LICENSE`).
