# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo build                      # Build the library
cargo test                       # Run all tests
cargo test <test_name>           # Run a specific test
cargo test -- --nocapture        # Run tests with stdout visible
cargo check                      # Quick compilation check
cargo doc --open                 # Generate and view documentation

# Feature-specific builds
cargo build --features monoio    # Build with monoio async support
cargo build --features tokio     # Build with tokio async support
cargo build --no-default-features # Build without TTL
```

## Feature Flags

- `ttl` (default): Enables time-to-live support for entries (activates `hislab/tagged`, `hislab/rand`, `rand`)
- `monoio`: Async integration for monoio (single-thread, io_uring) - implies `ttl`
- `tokio`: Async integration for tokio (multi-thread) - implies `ttl`

> `monoio` and `tokio` are mutually exclusive (compile_error! if both enabled)

## Architecture Overview

OxidArt is an Adaptive Radix Tree (ART) implementation optimized for O(k) key-value operations where k is key length.

### Core Components

**`OxidArt` struct (lib.rs)** - Main tree structure using HiSlab allocation for nodes:
- Uses `HiSlab<Node>` (custom hierarchical bitmap slab) for O(1) insert/remove
- Separate HiSlab (`child_list`) for overflow child pointers
- With TTL: maintains `now: u64` timestamp for expiration checks

**`Node` structure** - Changes based on TTL feature:
- With TTL: `compression: SmallVec<[u8; 8]>`, `val: Option<(Bytes, u64)>`, `parent_idx: u32`, `parent_radix: u8`
- Without TTL: `compression: SmallVec<[u8; 23]>`, `val: Option<Bytes>`
- Both use `Childs` for child management

**Two-tier child storage (node_childs.rs)**:
- `Childs`: Inline storage for up to 10 children (64-byte aligned)
- `HugeChilds`: Overflow storage for remaining 117 possible radix values
- Automatic promotion when inline capacity exceeded

**HiSlab integration (with TTL)**:
- `insert()`: Regular insert for nodes without TTL
- `insert_tagged()`: Tagged insert for nodes with TTL (enables O(1) random sampling)
- `random_tagged()`: Select random node with TTL for probabilistic eviction

**Async modules (monoio.rs, tokio.rs)**:
- `tick()`: Updates internal timestamp to current system time
- `spawn_ticker()`: Spawns background task for periodic timestamp updates
- `spawn_evictor()`: Spawns background task for periodic TTL eviction

### Key Algorithms

- **Path compression**: Single-child paths collapse into parent's compression vector
- **Automatic recompression**: After deletions, tree reshapes by absorbing single-child nodes (updates `parent_idx` of grandchildren)
- **Prefix operations**: `getn`/`deln` traverse to prefix then collect/delete all descendants
- **Lazy TTL expiration**: Expired entries filtered on access (get/getn)
- **Active TTL eviction**: Redis-style probabilistic sampling via `evict_expired()`

### Public API

| Method | Description |
|--------|-------------|
| `new()` | Create empty tree |
| `shared_with_ticker(interval)` | Create shared tree with auto-ticker |
| `shared_with_evictor(tick_interval, evict_interval)` | Create shared tree with ticker + evictor (recommended) |
| `get(key)` | Exact key lookup |
| `set(key, val)` | Insert/update without expiration |
| `set_ttl(key, duration, val)` | Insert/update with TTL |
| `del(key)` | Delete exact key, returns old value |
| `getn(prefix)` | Get all key-value pairs matching prefix |
| `deln(prefix)` | Delete all entries matching prefix, returns count |
| `get_ttl(key)` | Get TTL status of a key (returns `TtlResult`) |
| `expire(key, duration)` | Set TTL on existing key |
| `persist(key)` | Remove TTL from key (make permanent) |
| `set_now(timestamp)` | Update internal clock |
| `tick()` | Update clock to current time |
| `evict_expired()` | Redis-style TTL eviction |

### TtlResult Enum

```rust
pub enum TtlResult {
    KeyNotExist,        // Key doesn't exist or is expired
    KeyWithTtl(u64),    // Key exists with remaining seconds
    KeyWithoutTtl,      // Key exists, no TTL (permanent)
}
```

### Recommended Usage (Production)

```rust
// monoio
let tree = OxidArt::shared_with_evictor(
    Duration::from_millis(100),  // tick interval
    Duration::from_secs(1),       // eviction interval
);

// tokio
let tree = OxidArt::shared_with_evictor(
    Duration::from_millis(100),
    Duration::from_secs(1),
).await;
```

### TTL Eviction Strategy

The `evict_expired()` method implements Redis-style probabilistic eviction:
1. Sample 20 random entries with TTL (via `random_tagged`)
2. Delete expired entries using stored `parent_idx`/`parent_radix`
3. If >= 5 (25%) were expired, repeat
4. Stop when < 25% expired or no more tagged entries

This provides O(1) amortized cleanup without scanning the entire tree.

### Test Data

`list.txt` contains a French word list (~350K words) used for large-scale integration tests.
