# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo build                    # Build the library
cargo test                     # Run all tests
cargo test <test_name>         # Run a specific test
cargo test -- --nocapture      # Run tests with stdout visible
cargo check                    # Quick compilation check
cargo doc --open               # Generate and view documentation
```

## Feature Flags

- `ttl` (default): Enables time-to-live support for entries. Currently under development on the `ttl` branch.

Build without TTL: `cargo build --no-default-features`

## Architecture Overview

OxidArt is an Adaptive Radix Tree (ART) implementation optimized for O(k) key-value operations where k is key length.

### Core Components

**`OxidArt` struct (lib.rs)** - Main tree structure using slab allocation for nodes:
- Pre-allocates 1024 node capacity on creation
- Uses `Slab<Node>` for memory-efficient node storage
- Separate slab (`child_list`) for overflow child pointers

**`Node` structure** - Changes based on TTL feature:
- With TTL: `compression: SmallVec<[u8; 8]>`, `val: Option<(Bytes, u64)>`
- Without TTL: `compression: SmallVec<[u8; 23]>`, `val: Option<Bytes>`
- Both use `Childs` for child management

**Two-tier child storage (node_childs.rs)**:
- `Childs`: Inline storage for up to 10 children (64-byte aligned)
- `HugeChilds`: Overflow storage for remaining 117 possible radix values
- Automatic promotion when inline capacity exceeded

### Key Algorithms

- **Path compression**: Single-child paths collapse into parent's compression vector
- **Automatic recompression**: After deletions, tree reshapes by absorbing single-child nodes
- **Prefix operations**: `getn`/`deln` traverse to prefix then collect/delete all descendants

### Public API

| Method | Description |
|--------|-------------|
| `get(key)` | Exact key lookup |
| `set(key, ttl, val)` | Insert/update (ttl param only with TTL feature) |
| `del(key)` | Delete exact key, returns old value |
| `getn(prefix)` | Get all key-value pairs matching prefix |
| `deln(prefix)` | Delete all entries matching prefix, returns count |

### Test Data

`list.txt` contains a French word list (~350K words) used for large-scale integration tests.
