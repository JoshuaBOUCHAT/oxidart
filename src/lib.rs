//! # OxidArt
//!
//! A high-performance, compressed Adaptive Radix Tree (ART) implementation in Rust
//! for fast key-value storage operations.
//!
//! ## Features
//!
//! - **O(k) operations**: All operations (get, set, del) run in O(k) time where k is the key length
//! - **Path compression**: Minimizes memory usage by compressing single-child paths
//! - **Prefix operations**: Supports `getn` and `deln` for prefix-based queries and deletions
//! - **Zero-copy values**: Uses `bytes::Bytes` for efficient value handling
//!
//! ## Example
//!
//! ```rust
//! use oxidart::OxidArt;
//! use bytes::Bytes;
//!
//! let mut tree = OxidArt::new();
//!
//! // Insert key-value pairs
//! tree.set(Bytes::from_static(b"hello"), Bytes::from_static(b"world"));
//! tree.set(Bytes::from_static(b"hello:foo"), Bytes::from_static(b"bar"));
//!
//! // Retrieve a value
//! assert_eq!(tree.get(Bytes::from_static(b"hello")), Some(Bytes::from_static(b"world")));
//!
//! // Get all entries with a prefix
//! let entries = tree.getn(Bytes::from_static(b"hello"));
//! assert_eq!(entries.len(), 2);
//!
//! // Delete a key
//! let deleted = tree.del(Bytes::from_static(b"hello"));
//! assert_eq!(deleted, Some(Bytes::from_static(b"world")));
//!
//! // Delete all keys with a prefix
//! let count = tree.deln(Bytes::from_static(b"hello"));
//! assert_eq!(count, 1);
//! ```
//!
//! ## Key Requirements
//!
//! Keys must be valid ASCII bytes. Non-ASCII keys will trigger a debug assertion.

mod node_childs;
#[cfg(test)]
mod test;

use bytes::Bytes;
use slab::Slab;
use smallvec::SmallVec;

use crate::node_childs::ChildAble;
use crate::node_childs::Childs;
use crate::node_childs::HugeChilds;

/// A compressed Adaptive Radix Tree for fast key-value storage.
///
/// `OxidArt` provides O(k) time complexity for all operations where k is the key length.
/// It uses path compression to minimize memory footprint while maintaining high performance.
///
/// # Example
///
/// ```rust
/// use oxidart::OxidArt;
/// use bytes::Bytes;
///
/// let mut tree = OxidArt::new();
/// tree.set(Bytes::from_static(b"key"), Bytes::from_static(b"value"));
///
/// assert_eq!(tree.get(Bytes::from_static(b"key")), Some(Bytes::from_static(b"value")));
/// ```
pub struct OxidArt {
    pub(crate) map: Slab<Node>,
    pub(crate) child_list: Slab<HugeChilds>,
    versions: Vec<u32>,
    root_idx: u32,
}
impl Default for OxidArt {
    fn default() -> Self {
        Self::new()
    }
}

impl OxidArt {
    /// Creates a new empty `OxidArt` tree.
    ///
    /// The tree is pre-allocated with capacity for 1024 nodes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidart::OxidArt;
    ///
    /// let tree = OxidArt::new();
    /// ```
    pub fn new() -> Self {
        let mut map = Slab::with_capacity(1024);

        let root_idx = map.insert(Node::default()) as u32;
        let versions = vec![root_idx];
        let child_list = Slab::with_capacity(32);

        Self {
            map,
            root_idx,
            versions,
            child_list,
        }
    }
    fn insert(&mut self, node: Node) -> u32 {
        let idx = self.map.insert(node) as u32;
        if self.versions.len() == idx as usize {
            self.versions.push(0);
        } else {
            self.versions[idx as usize] += 1;
        }
        idx
    }
    fn get_node(&self, idx: u32) -> &Node {
        self.try_get_node(idx)
            .expect("Call to unfailable get_node failed")
    }
    fn get_node_mut(&mut self, idx: u32) -> &mut Node {
        self.try_get_node_mut(idx)
            .expect("Call to unfailable get_node failed")
    }

    fn try_get_node(&self, idx: u32) -> Option<&Node> {
        self.map.get(idx as usize)
    }
    fn try_get_node_mut(&mut self, idx: u32) -> Option<&mut Node> {
        self.map.get_mut(idx as usize)
    }
    fn find(&self, idx: u32, radix: u8) -> Option<u32> {
        let child = &self.try_get_node(idx)?.childs;

        if let Some(index) = child.find(radix) {
            return Some(index);
        }
        self.child_list
            .get(child.get_next_idx()? as usize)?
            .find(radix)
    }
    fn intiate_new_huge_child(&mut self, radix: u8, idx: u32) -> u32 {
        self.child_list.insert(HugeChilds::new(radix, idx)) as u32
    }
}
impl OxidArt {
    /// Retrieves the value associated with the given key.
    ///
    /// Returns `Some(value)` if the key exists, or `None` if it doesn't.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up. Must be valid ASCII.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidart::OxidArt;
    /// use bytes::Bytes;
    ///
    /// let mut tree = OxidArt::new();
    /// tree.set(Bytes::from_static(b"hello"), Bytes::from_static(b"world"));
    ///
    /// assert_eq!(tree.get(Bytes::from_static(b"hello")), Some(Bytes::from_static(b"world")));
    /// assert_eq!(tree.get(Bytes::from_static(b"missing")), None);
    /// ```
    pub fn get(&self, key: Bytes) -> Option<Bytes> {
        debug_assert!(key.is_ascii(), "key must be ASCII");
        let key_len = key.len();
        if key_len == 0 {
            return self.try_get_node(self.root_idx)?.val.clone();
        }

        let mut idx = self.root_idx;
        let mut cursor = 0;

        loop {
            idx = self.find(idx, key[cursor])?;
            let node = self.try_get_node(idx)?;
            // Entering the node, increment cursor by 1
            cursor += 1;
            match node.compare_compression_key(&key[cursor..]) {
                CompResult::Final => return node.val.clone(),
                CompResult::Partial(_) => return None,
                CompResult::Path => {
                    cursor += node.compression.len();
                }
            }
        }
    }

    /// Returns all key-value pairs where the key starts with the given prefix.
    ///
    /// If the prefix is empty, returns all entries in the tree.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix to match. Must be valid ASCII.
    ///
    /// # Returns
    ///
    /// A vector of `(key, value)` tuples for all matching entries.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidart::OxidArt;
    /// use bytes::Bytes;
    ///
    /// let mut tree = OxidArt::new();
    /// tree.set(Bytes::from_static(b"user:1"), Bytes::from_static(b"alice"));
    /// tree.set(Bytes::from_static(b"user:2"), Bytes::from_static(b"bob"));
    /// tree.set(Bytes::from_static(b"post:1"), Bytes::from_static(b"hello"));
    ///
    /// let users = tree.getn(Bytes::from_static(b"user:"));
    /// assert_eq!(users.len(), 2);
    /// ```
    pub fn getn(&self, prefix: Bytes) -> Vec<(Bytes, Bytes)> {
        debug_assert!(prefix.is_ascii(), "prefix must be ASCII");
        let mut results = Vec::new();
        let prefix_len = prefix.len();

        if prefix_len == 0 {
            self.collect_all(self.root_idx, Vec::new(), &mut results);
            return results;
        }

        // Traverse like get, tracking the actual path
        let mut idx = self.root_idx;
        let mut cursor = 0;
        let mut key_path: Vec<u8> = Vec::new();

        loop {
            let radix = prefix[cursor];
            let Some(child_idx) = self.find(idx, radix) else {
                return results;
            };
            idx = child_idx;
            key_path.push(radix);

            let Some(node) = self.try_get_node(idx) else {
                return results;
            };
            cursor += 1;

            match node.compare_compression_key(&prefix[cursor..]) {
                CompResult::Final => {
                    // Exact prefix found
                    key_path.extend_from_slice(&node.compression);
                    self.collect_all_from(idx, key_path, &mut results);
                    return results;
                }
                CompResult::Partial(common_len) => {
                    let prefix_rest_len = prefix_len - cursor;
                    if common_len == prefix_rest_len {
                        // Prefix ends within the compression
                        key_path.extend_from_slice(&node.compression);
                        self.collect_all_from(idx, key_path, &mut results);
                    }
                    return results;
                }
                CompResult::Path => {
                    key_path.extend_from_slice(&node.compression);
                    cursor += node.compression.len();
                }
            }
        }
    }

    /// Collects from a node whose key is already complete in key_path
    fn collect_all_from(
        &self,
        node_idx: u32,
        key_path: Vec<u8>,
        results: &mut Vec<(Bytes, Bytes)>,
    ) {
        let Some(node) = self.try_get_node(node_idx) else {
            return;
        };

        if let Some(val) = &node.val {
            results.push((Bytes::from(key_path.clone()), val.clone()));
        }

        self.iter_all_children(node_idx, |radix, child_idx| {
            let mut child_key = key_path.clone();
            child_key.push(radix);
            self.collect_all(child_idx, child_key, results);
        });
    }

    /// Recursively collects, adding the node's compression
    fn collect_all(
        &self,
        node_idx: u32,
        mut key_prefix: Vec<u8>,
        results: &mut Vec<(Bytes, Bytes)>,
    ) {
        let Some(node) = self.try_get_node(node_idx) else {
            return;
        };

        key_prefix.extend_from_slice(&node.compression);

        if let Some(val) = &node.val {
            results.push((Bytes::from(key_prefix.clone()), val.clone()));
        }

        self.iter_all_children(node_idx, |radix, child_idx| {
            let mut child_key = key_prefix.clone();
            child_key.push(radix);
            self.collect_all(child_idx, child_key, results);
        });
    }

    /// Iterates over all children of a node (childs + huge_childs)
    fn iter_all_children<F>(&self, node_idx: u32, mut f: F)
    where
        F: FnMut(u8, u32),
    {
        let Some(node) = self.try_get_node(node_idx) else {
            return;
        };

        for (radix, child_idx) in node.childs.iter() {
            f(radix, child_idx);
        }

        if let Some(huge_idx) = node.childs.get_next_idx()
            && let Some(huge_childs) = self.child_list.get(huge_idx as usize)
        {
            for (radix, child_idx) in huge_childs.iter() {
                f(radix, child_idx);
            }
        }
    }

    /// Inserts or updates a key-value pair in the tree.
    ///
    /// If the key already exists, the value is replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert. Must be valid ASCII.
    /// * `val` - The value to associate with the key.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidart::OxidArt;
    /// use bytes::Bytes;
    ///
    /// let mut tree = OxidArt::new();
    ///
    /// // Insert a new key
    /// tree.set(Bytes::from_static(b"key"), Bytes::from_static(b"value1"));
    ///
    /// // Update an existing key
    /// tree.set(Bytes::from_static(b"key"), Bytes::from_static(b"value2"));
    ///
    /// assert_eq!(tree.get(Bytes::from_static(b"key")), Some(Bytes::from_static(b"value2")));
    /// ```
    pub fn set(&mut self, key: Bytes, val: Bytes) {
        debug_assert!(key.is_ascii(), "key must be ASCII");
        let key_len = key.len();
        if key_len == 0 {
            self.get_node_mut(self.root_idx).set_val(val);
            return;
        }
        let mut idx = self.root_idx;
        let mut cursor = 0;

        loop {
            let Some(child_idx) = self.find(idx, key[cursor]) else {
                self.create_node_with_val(idx, key[cursor], val, &key[(cursor + 1)..]);
                return;
            };
            idx = child_idx;
            // Entering the node, increment cursor by 1
            cursor += 1;
            let node_comparaison = self.get_node(idx).compare_compression_key(&key[cursor..]);
            let common_len = match node_comparaison {
                CompResult::Final => {
                    self.get_node_mut(idx).set_val(val);
                    return;
                }
                CompResult::Path => {
                    cursor += self.get_node(idx).compression.len();
                    continue;
                }
                CompResult::Partial(common_len) => common_len,
            };

            // Split: node compression only partially matches the key
            let key_rest = &key[cursor..];
            let val_on_intermediate = common_len == key_rest.len();

            // Extract old state and configure intermediate in one pass
            let (old_compression, old_val, old_childs) = {
                let node = self.get_node_mut(idx);
                let old_compression = std::mem::take(&mut node.compression);
                let old_val = node.val.take();
                let old_childs = std::mem::take(&mut node.childs);

                node.compression = SmallVec::from_slice(&old_compression[..common_len]);
                if val_on_intermediate {
                    node.val = Some(val.clone());
                }

                (old_compression, old_val, old_childs)
            };

            // Create a node for the old content
            let old_radix = old_compression[common_len];
            let old_child = Node {
                compression: SmallVec::from_slice(&old_compression[common_len + 1..]),
                val: old_val,
                childs: old_childs,
            };
            let old_child_idx = self.insert(old_child);
            self.get_node_mut(idx).childs.push(old_radix, old_child_idx);

            // If the value doesn't go on the intermediate node, create a new leaf
            if !val_on_intermediate {
                let new_radix = key_rest[common_len];
                let new_compression = &key_rest[common_len + 1..];
                self.create_node_with_val(idx, new_radix, val, new_compression);
            }

            return;
        }
    }
    fn create_node_with_val(&mut self, idx: u32, radix: u8, val: Bytes, compression: &[u8]) {
        let (is_full, huge_child_idx) = {
            let father_node = self.get_node(idx);
            (
                father_node.childs.is_full(),
                father_node.get_huge_childs_idx(),
            )
        };
        let new_leaf = Node::new_leaf(compression, val);
        let inserted_idx = self.insert(new_leaf);
        match (is_full, huge_child_idx) {
            (false, _) => {
                self.get_node_mut(idx).childs.push(radix, inserted_idx);
            }
            (true, None) => {
                let new_child_idx = self.intiate_new_huge_child(radix, inserted_idx);
                self.get_node_mut(idx).childs.set_new_childs(new_child_idx);
            }
            (true, Some(huge_idx)) => {
                self.child_list
                    .get_mut(huge_idx as usize)
                    .expect("if key exist childs should too")
                    .push(radix, inserted_idx);
            }
        }
    }

    /// Deletes a key from the tree and returns its value.
    ///
    /// Returns `Some(value)` if the key existed, or `None` if it didn't.
    /// The tree automatically recompresses paths after deletion.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete. Must be valid ASCII.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidart::OxidArt;
    /// use bytes::Bytes;
    ///
    /// let mut tree = OxidArt::new();
    /// tree.set(Bytes::from_static(b"key"), Bytes::from_static(b"value"));
    ///
    /// let deleted = tree.del(Bytes::from_static(b"key"));
    /// assert_eq!(deleted, Some(Bytes::from_static(b"value")));
    ///
    /// // Key no longer exists
    /// assert_eq!(tree.get(Bytes::from_static(b"key")), None);
    /// ```
    pub fn del(&mut self, key: Bytes) -> Option<Bytes> {
        debug_assert!(key.is_ascii(), "key must be ASCII");
        let key_len = key.len();
        if key_len == 0 {
            let old_val = self.get_node_mut(self.root_idx).val.take();
            self.try_recompress(self.root_idx);
            return old_val;
        }

        // Traverse like get, keeping track of the immediate parent
        let mut parent_idx = self.root_idx;
        let mut parent_radix = key[0];
        let mut idx = self.find(parent_idx, parent_radix)?;
        let mut cursor = 1;

        let target_idx = loop {
            let node = self.try_get_node(idx)?;
            match node.compare_compression_key(&key[cursor..]) {
                CompResult::Final => break idx,
                CompResult::Partial(_) => return None,
                CompResult::Path => {
                    cursor += node.compression.len();
                }
            }

            // Continue traversal
            parent_idx = idx;
            parent_radix = key[cursor];
            idx = self.find(idx, parent_radix)?;
            cursor += 1;
        };

        // Check if the node has children
        let has_children = {
            let node = self.get_node(target_idx);
            !node.childs.is_empty() || node.childs.get_next_idx().is_some()
        };

        if has_children {
            // Node with children: keep the node, just remove the value
            let old_val = self.get_node_mut(target_idx).val.take()?;
            // Try recompression (absorb the single child if possible)
            self.try_recompress(target_idx);
            Some(old_val)
        } else {
            // Node without children (leaf): completely remove from the slab
            let node = self.map.remove(target_idx as usize);
            let old_val = node.val?;
            self.remove_child(parent_idx, parent_radix);
            // Try recompression on the parent (except root)
            if parent_idx != self.root_idx {
                self.try_recompress(parent_idx);
            }
            Some(old_val)
        }
    }

    /// Deletes all keys that start with the given prefix.
    ///
    /// Returns the number of key-value pairs that were deleted.
    /// If the prefix is empty, all entries are deleted.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix to match. Must be valid ASCII.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidart::OxidArt;
    /// use bytes::Bytes;
    ///
    /// let mut tree = OxidArt::new();
    /// tree.set(Bytes::from_static(b"user:1"), Bytes::from_static(b"alice"));
    /// tree.set(Bytes::from_static(b"user:2"), Bytes::from_static(b"bob"));
    /// tree.set(Bytes::from_static(b"post:1"), Bytes::from_static(b"hello"));
    ///
    /// // Delete all user entries
    /// let count = tree.deln(Bytes::from_static(b"user:"));
    /// assert_eq!(count, 2);
    ///
    /// // Only post entries remain
    /// assert_eq!(tree.getn(Bytes::from_static(b"")).len(), 1);
    /// ```
    pub fn deln(&mut self, prefix: Bytes) -> usize {
        debug_assert!(prefix.is_ascii(), "prefix must be ASCII");
        let prefix_len = prefix.len();

        if prefix_len == 0 {
            // Delete everything from root (keep root node, clear its content)
            let root = self.get_node_mut(self.root_idx);
            let had_val = root.val.take().is_some();
            let childs_to_free: Vec<u32> = self.collect_child_indices(self.root_idx);

            // Clear children of root (note: root's huge_childs not freed, negligible)
            self.get_node_mut(self.root_idx).childs = Childs::default();

            let freed = self.free_subtree_iterative(childs_to_free);
            return freed + if had_val { 1 } else { 0 };
        }

        // Traverse like del
        let mut parent_idx = self.root_idx;
        let mut parent_radix = prefix[0];
        let Some(mut idx) = self.find(parent_idx, parent_radix) else {
            return 0;
        };
        let mut cursor = 1;

        let target_idx = loop {
            let Some(node) = self.try_get_node(idx) else {
                return 0;
            };

            match node.compare_compression_key(&prefix[cursor..]) {
                CompResult::Final => break idx,
                CompResult::Partial(common_len) => {
                    // Does the prefix end within the compression?
                    let prefix_rest_len = prefix_len - cursor;
                    if common_len == prefix_rest_len {
                        break idx;
                    }
                    // Divergence, nothing to delete
                    return 0;
                }
                CompResult::Path => {
                    cursor += node.compression.len();
                }
            }

            // Continue traversal
            parent_idx = idx;
            parent_radix = prefix[cursor];
            let Some(child_idx) = self.find(idx, parent_radix) else {
                return 0;
            };
            idx = child_idx;
            cursor += 1;
        };

        // Cut the link from parent
        self.remove_child(parent_idx, parent_radix);

        // Free the entire subtree (iterative DFS)
        let count = self.free_subtree_iterative(vec![target_idx]);

        // Recompression of parent (except root since get doesn't handle root with compression)
        if parent_idx != self.root_idx {
            self.try_recompress(parent_idx);
        }

        count
    }

    /// Collects all child indices of a node
    fn collect_child_indices(&self, node_idx: u32) -> Vec<u32> {
        let mut indices = Vec::new();
        let Some(node) = self.try_get_node(node_idx) else {
            return indices;
        };

        for (_, child_idx) in node.childs.iter() {
            indices.push(child_idx);
        }

        if let Some(huge_idx) = node.childs.get_next_idx()
            && let Some(huge_childs) = self.child_list.get(huge_idx as usize)
        {
            for (_, child_idx) in huge_childs.iter() {
                indices.push(child_idx);
            }
        }

        indices
    }

    /// Frees a subtree iteratively (DFS), returns the number of deleted values
    fn free_subtree_iterative(&mut self, initial_nodes: Vec<u32>) -> usize {
        let mut stack = initial_nodes;
        let mut count = 0;

        while let Some(node_idx) = stack.pop() {
            // Collect children before removing the node
            let (children, has_val, huge_child_idx) = {
                let Some(node) = self.try_get_node(node_idx) else {
                    continue;
                };

                let mut children: Vec<u32> = node.childs.iter().map(|(_, idx)| idx).collect();

                let huge_idx = node.childs.get_next_idx();
                if let Some(hi) = huge_idx
                    && let Some(huge_childs) = self.child_list.get(hi as usize)
                {
                    children.extend(huge_childs.iter().map(|(_, idx)| idx));
                }

                (children, node.val.is_some(), huge_idx)
            };

            // Add children to the stack
            stack.extend(children);

            // Count if it had a value
            if has_val {
                count += 1;
            }

            // Remove huge_childs if present
            if let Some(huge_idx) = huge_child_idx {
                self.child_list.remove(huge_idx as usize);
            }

            // Remove the node from the slab
            self.map.remove(node_idx as usize);
        }

        count
    }

    /// If the node has exactly 1 child and no value, absorb the child
    fn try_recompress(&mut self, node_idx: u32) {
        let node = self.get_node(node_idx);
        if node.val.is_some() {
            return;
        }

        let Some((child_radix, child_idx)) = node.childs.get_single_child() else {
            return;
        };

        // Absorb the child: compression = current + radix + child.compression
        let child = self.map.remove(child_idx as usize);
        let node = self.get_node_mut(node_idx);

        node.compression.push(child_radix);
        node.compression.extend_from_slice(&child.compression);
        node.val = child.val;
        node.childs = child.childs;
    }

    fn remove_child(&mut self, parent_idx: u32, radix: u8) {
        let parent = self.get_node_mut(parent_idx);
        if parent.childs.remove(radix).is_some() {
            return;
        }
        // Otherwise it's in huge_childs
        if let Some(huge_idx) = parent.childs.get_next_idx() {
            self.child_list
                .get_mut(huge_idx as usize)
                .expect("huge_childs should exist")
                .remove(radix);
        }
    }
}

#[derive(Default)]
struct Node {
    compression: SmallVec<[u8; 23]>,
    val: Option<Bytes>,
    childs: Childs,
}
enum CompResult {
    ///The compresion completely part of the key need travel for more
    Path,
    Final,
    Partial(usize),
}

impl Node {
    fn compare_compression_key(&self, key_rest: &[u8]) -> CompResult {
        use std::cmp::Ordering::*;
        match self.compression.len().cmp(&key_rest.len()) {
            Equal => {
                let common_len = self.get_common_len(key_rest);
                if common_len == key_rest.len() {
                    CompResult::Final
                } else {
                    CompResult::Partial(common_len)
                }
            }
            Greater => CompResult::Partial(self.get_common_len(key_rest)),
            Less => {
                let common_len = self.get_common_len(key_rest);
                if common_len == self.compression.len() {
                    CompResult::Path
                } else {
                    CompResult::Partial(common_len)
                }
            }
        }
    }
    #[allow(clippy::needless_range_loop)]
    fn get_common_len(&self, key_rest: &[u8]) -> usize {
        let len = self.compression.len().min(key_rest.len());

        for i in 0..len {
            if self.compression[i] != key_rest[i] {
                return i;
            }
        }
        len
    }
    fn set_val(&mut self, val: Bytes) {
        self.val = Some(val)
    }
    fn get_huge_childs_idx(&self) -> Option<u32> {
        self.childs.get_next_idx()
    }
    fn new_leaf(compression: &[u8], val: Bytes) -> Self {
        Node {
            compression: SmallVec::from_slice(compression),
            val: Some(val),
            childs: Childs::default(),
        }
    }
}
