# OxidArt

A blazingly fast Adaptive Radix Tree (ART) implementation in Rust with path compression.

[![Crates.io](https://img.shields.io/crates/v/oxidart.svg)](https://crates.io/crates/oxidart)
[![Documentation](https://docs.rs/oxidart/badge.svg)](https://docs.rs/oxidart)
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

## Features

- **O(k) complexity** - All operations run in O(k) time where k is the key length, not the number of entries
- **Path compression** - Minimizes memory usage by collapsing single-child paths
- **Prefix queries** - `getn` and `deln` for efficient prefix-based operations
- **Zero-copy values** - Uses `bytes::Bytes` for efficient value handling
- **Memory efficient** - Adaptive node sizing with `SmallVec` and `Slab` allocation

## Installation

```toml
[dependencies]
oxidart = "0.1"
bytes = "1"
```

## Quick Start

```rust
use oxidart::OxidArt;
use bytes::Bytes;

let mut tree = OxidArt::new();

// Insert key-value pairs
tree.set(Bytes::from_static(b"hello"), Bytes::from_static(b"world"));
tree.set(Bytes::from_static(b"hello:foo"), Bytes::from_static(b"bar"));

// Retrieve a value
assert_eq!(tree.get(Bytes::from_static(b"hello")), Some(Bytes::from_static(b"world")));

// Get all entries with a prefix
let entries = tree.getn(Bytes::from_static(b"hello"));
assert_eq!(entries.len(), 2);

// Delete a key
tree.del(Bytes::from_static(b"hello"));

// Delete all keys with a prefix
tree.deln(Bytes::from_static(b"hello"));
```

## API

| Method | Description |
|--------|-------------|
| `new()` | Create a new empty tree |
| `get(key)` | Get value by exact key |
| `set(key, value)` | Insert or update a key-value pair |
| `del(key)` | Delete by exact key, returns the old value |
| `getn(prefix)` | Get all entries matching a prefix |
| `deln(prefix)` | Delete all entries matching a prefix |

## Why ART?

Adaptive Radix Trees combine the efficiency of radix trees with adaptive node sizes:

- Unlike hash maps, ART maintains key ordering and supports efficient range/prefix queries
- Unlike B-trees, ART has O(k) lookup independent of the number of entries
- Path compression eliminates redundant nodes, reducing memory overhead

## License

Licensed under the Mozilla Public License 2.0.
