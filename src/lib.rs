mod smallstr;

use std::u32;

use arrayvec::ArrayString;
use arrayvec::ArrayVec;
use bytes::Bytes;
use compact_str::CompactString;
use slab::Slab;
use smallvec::SmallVec;

const CHILDS_SIZE: usize = 10;

pub struct OxidArt {
    map: Slab<Node>,
    child_list: Slab<Childs>,
    versions: Vec<u32>,
    root_idx: u32,
}
impl OxidArt {
    fn new() -> Self {
        let mut map = Slab::with_capacity(1024);

        let root_idx = map.insert(Node::default()) as u32;
        let versions = vec![root_idx];
        let child_list = Slab::with_capacity(1024);

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
    fn get_node(&self, idx: u32) -> Option<&Node> {
        self.map.get(idx as usize)
    }
    fn find(&self, idx: u32, radix: u8) -> Option<u32> {
        let mut child = &self.get_node(idx)?.childs;
        loop {
            if let Some(index) = child.find(radix) {
                return Some(index as u32);
            }
            child = self.child_list.get(child.get_next_idx()? as usize)?;
        }
    }
}
impl OxidArt {
    pub fn get(&self, key: Bytes) -> Option<Bytes> {
        let key_len = key.len();
        if key_len == 0 {
            return self.get_node(self.root_idx)?.val.clone();
        }

        let mut idx = self.root_idx;
        let mut cursor = 0;

        loop {
            idx = self.find(idx, key[cursor])?;
            let node = self.get_node(idx)?;
            //on passe dans le node donc le cursor augmente de 1
            cursor += 1;
            match node.compare_compression_key(&key[cursor..]) {
                CompResult::CompIsFinal => return node.val.clone(),
                CompResult::CompIsPartial(_) => return None,
                CompResult::CompIsPath => {
                    cursor += node.compression.len();
                }
            }
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
    CompIsPath,
    CompIsFinal,
    CompIsPartial(usize),
}

impl Node {
    fn compare_compression_key(&self, key_rest: &[u8]) -> CompResult {
        use std::cmp::Ordering::*;
        match self.compression.len().cmp(&key_rest.len()) {
            Equal => {
                let common_len = self.get_common_len(key_rest);
                if common_len == key_rest.len() {
                    CompResult::CompIsFinal
                } else {
                    CompResult::CompIsPartial(common_len)
                }
            }
            Greater => CompResult::CompIsPartial(self.get_common_len(key_rest)),
            Less => {
                let common_len = self.get_common_len(key_rest);
                if common_len == self.compression.len() {
                    CompResult::CompIsPath
                } else {
                    CompResult::CompIsPartial(common_len)
                }
            }
        }
    }
    fn get_common_len(&self, key_rest: &[u8]) -> usize {
        let len = self.compression.len().min(key_rest.len());
        for i in 0..len {
            if self.compression[i] != key_rest[i] {
                return 0;
            }
        }
        len
    }
}

struct Childs {
    idxs: ArrayVec<u32, CHILDS_SIZE>,
    radixs: ArrayVec<u8, CHILDS_SIZE>,
    maybe_next_childs_idx: u32,
}
impl Default for Childs {
    fn default() -> Self {
        Self {
            maybe_next_childs_idx: u32::MAX,
            idxs: ArrayVec::default(),
            radixs: ArrayVec::default(),
        }
    }
}
impl Childs {
    fn get_next_idx(&self) -> Option<u32> {
        if self.maybe_next_childs_idx == u32::MAX {
            None
        } else {
            Some(self.maybe_next_childs_idx)
        }
    }
    fn find(&self, radix: u8) -> Option<usize> {
        self.radixs.iter().position(|&c| c == radix)
    }
}
