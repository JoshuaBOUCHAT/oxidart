mod node_childs;
#[cfg(test)]
mod test;
use std::u32;

use bytes::Bytes;
use slab::Slab;
use smallvec::SmallVec;

use crate::node_childs::ChildAble;
use crate::node_childs::Childs;
use crate::node_childs::HugeChilds;

pub struct OxidArt {
    pub(crate) map: Slab<Node>,
    pub(crate) child_list: Slab<HugeChilds>,
    versions: Vec<u32>,
    root_idx: u32,
}
impl OxidArt {
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
            return Some(index as u32);
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
    pub fn get(&self, key: Bytes) -> Option<Bytes> {
        let key_len = key.len();
        if key_len == 0 {
            return self.try_get_node(self.root_idx)?.val.clone();
        }

        let mut idx = self.root_idx;
        let mut cursor = 0;

        loop {
            idx = self.find(idx, key[cursor])?;
            let node = self.try_get_node(idx)?;
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

    /// Retourne tous les (clé, valeur) dont la clé commence par `prefix`
    pub fn getn(&self, prefix: Bytes) -> Vec<(Bytes, Bytes)> {
        let mut results = Vec::new();
        let prefix_len = prefix.len();

        if prefix_len == 0 {
            self.collect_all(self.root_idx, Vec::new(), &mut results);
            return results;
        }

        // Parcours identique à get, on track le chemin réel
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
                CompResult::CompIsFinal => {
                    // Préfixe exact trouvé
                    key_path.extend_from_slice(&node.compression);
                    self.collect_all_from(idx, key_path, &mut results);
                    return results;
                }
                CompResult::CompIsPartial(common_len) => {
                    let prefix_rest_len = prefix_len - cursor;
                    if common_len == prefix_rest_len {
                        // Préfixe se termine dans la compression
                        key_path.extend_from_slice(&node.compression);
                        self.collect_all_from(idx, key_path, &mut results);
                    }
                    return results;
                }
                CompResult::CompIsPath => {
                    key_path.extend_from_slice(&node.compression);
                    cursor += node.compression.len();
                }
            }
        }
    }

    /// Collecte depuis un node dont la clé est déjà complète dans key_path
    fn collect_all_from(&self, node_idx: u32, key_path: Vec<u8>, results: &mut Vec<(Bytes, Bytes)>) {
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

    /// Collecte récursivement, ajoute la compression du node
    fn collect_all(&self, node_idx: u32, mut key_prefix: Vec<u8>, results: &mut Vec<(Bytes, Bytes)>) {
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

    /// Itère sur tous les enfants d'un node (childs + huge_childs)
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

        if let Some(huge_idx) = node.childs.get_next_idx() {
            if let Some(huge_childs) = self.child_list.get(huge_idx as usize) {
                for (radix, child_idx) in huge_childs.iter() {
                    f(radix, child_idx);
                }
            }
        }
    }
    pub fn set(&mut self, key: Bytes, val: Bytes) {
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
            //on passe dans le node donc le cursor augmente de 1
            cursor += 1;
            let node_comparaison = self.get_node(idx).compare_compression_key(&key[cursor..]);
            let common_len = match node_comparaison {
                CompResult::CompIsFinal => {
                    self.get_node_mut(idx).set_val(val);
                    return;
                }
                CompResult::CompIsPath => {
                    cursor += self.get_node(idx).compression.len();
                    continue;
                }
                CompResult::CompIsPartial(common_len) => common_len,
            };

            // Split: la compression du node ne match que partiellement la clé
            let key_rest = &key[cursor..];
            let val_on_intermediate = common_len == key_rest.len();

            // Extraire l'ancien état et configurer l'intermédiaire en une passe
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

            // Créer un node pour l'ancien contenu
            let old_radix = old_compression[common_len];
            let old_child = Node {
                compression: SmallVec::from_slice(&old_compression[common_len + 1..]),
                val: old_val,
                childs: old_childs,
            };
            let old_child_idx = self.insert(old_child);
            self.get_node_mut(idx).childs.push(old_radix, old_child_idx);

            // Si la valeur ne va pas sur l'intermédiaire, créer un nouveau leaf
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
    pub fn del(&mut self, key: Bytes) -> Option<Bytes> {
        let key_len = key.len();
        if key_len == 0 {
            let old_val = self.get_node_mut(self.root_idx).val.take();
            self.try_recompress(self.root_idx);
            return old_val;
        }

        // Parcours comme get, on garde juste le parent immédiat
        let mut parent_idx = self.root_idx;
        let mut parent_radix = key[0];
        let mut idx = self.find(parent_idx, parent_radix)?;
        let mut cursor = 1;

        let target_idx = loop {
            let node = self.try_get_node(idx)?;
            match node.compare_compression_key(&key[cursor..]) {
                CompResult::CompIsFinal => break idx,
                CompResult::CompIsPartial(_) => return None,
                CompResult::CompIsPath => {
                    cursor += node.compression.len();
                }
            }

            // Continuer la traversée
            parent_idx = idx;
            parent_radix = key[cursor];
            idx = self.find(idx, parent_radix)?;
            cursor += 1;
        };

        // Check si le node a des enfants
        let has_children = {
            let node = self.get_node(target_idx);
            !node.childs.is_empty() || node.childs.get_next_idx().is_some()
        };

        if has_children {
            // Node avec enfants: on garde le node, juste supprimer la valeur
            let old_val = self.get_node_mut(target_idx).val.take()?;
            // Tenter recompression (absorbe l'unique enfant si possible)
            self.try_recompress(target_idx);
            Some(old_val)
        } else {
            // Node sans enfants (leaf): on supprime complètement le node de la slab
            let node = self.map.remove(target_idx as usize);
            let old_val = node.val?;
            self.remove_child(parent_idx, parent_radix);
            // Tenter recompression sur le parent (sauf root)
            if parent_idx != self.root_idx {
                self.try_recompress(parent_idx);
            }
            Some(old_val)
        }
    }

    /// Supprime toutes les clés commençant par `prefix`, retourne le nombre supprimé
    pub fn deln(&mut self, prefix: Bytes) -> usize {
        let prefix_len = prefix.len();

        if prefix_len == 0 {
            // Supprimer tout depuis la racine (garder le node racine, vider son contenu)
            let root = self.get_node_mut(self.root_idx);
            let had_val = root.val.take().is_some();
            let childs_to_free: Vec<u32> = self.collect_child_indices(self.root_idx);

            // Vider les enfants de la racine (note: huge_childs de root pas libéré, négligeable)
            self.get_node_mut(self.root_idx).childs = Childs::default();

            let freed = self.free_subtree_iterative(childs_to_free);
            return freed + if had_val { 1 } else { 0 };
        }

        // Parcours comme del
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
                CompResult::CompIsFinal => break idx,
                CompResult::CompIsPartial(common_len) => {
                    // Le préfixe se termine dans la compression?
                    let prefix_rest_len = prefix_len - cursor;
                    if common_len == prefix_rest_len {
                        break idx;
                    }
                    // Divergence, rien à supprimer
                    return 0;
                }
                CompResult::CompIsPath => {
                    cursor += node.compression.len();
                }
            }

            // Continuer la traversée
            parent_idx = idx;
            parent_radix = prefix[cursor];
            let Some(child_idx) = self.find(idx, parent_radix) else {
                return 0;
            };
            idx = child_idx;
            cursor += 1;
        };

        // Couper le lien depuis le parent
        self.remove_child(parent_idx, parent_radix);

        // Libérer tout le sous-arbre (DFS itératif)
        let count = self.free_subtree_iterative(vec![target_idx]);

        // Recompression du parent (sauf root car get ne gère pas root avec compression)
        if parent_idx != self.root_idx {
            self.try_recompress(parent_idx);
        }

        count
    }

    /// Collecte tous les indices d'enfants d'un node
    fn collect_child_indices(&self, node_idx: u32) -> Vec<u32> {
        let mut indices = Vec::new();
        let Some(node) = self.try_get_node(node_idx) else {
            return indices;
        };

        for (_, child_idx) in node.childs.iter() {
            indices.push(child_idx);
        }

        if let Some(huge_idx) = node.childs.get_next_idx() {
            if let Some(huge_childs) = self.child_list.get(huge_idx as usize) {
                for (_, child_idx) in huge_childs.iter() {
                    indices.push(child_idx);
                }
            }
        }

        indices
    }

    /// Libère un sous-arbre de manière itérative (DFS), retourne le nombre de valeurs supprimées
    fn free_subtree_iterative(&mut self, initial_nodes: Vec<u32>) -> usize {
        let mut stack = initial_nodes;
        let mut count = 0;

        while let Some(node_idx) = stack.pop() {
            // Collecter les enfants avant de supprimer le node
            let (children, has_val, huge_child_idx) = {
                let Some(node) = self.try_get_node(node_idx) else {
                    continue;
                };

                let mut children: Vec<u32> = node.childs.iter().map(|(_, idx)| idx).collect();

                let huge_idx = node.childs.get_next_idx();
                if let Some(hi) = huge_idx {
                    if let Some(huge_childs) = self.child_list.get(hi as usize) {
                        children.extend(huge_childs.iter().map(|(_, idx)| idx));
                    }
                }

                (children, node.val.is_some(), huge_idx)
            };

            // Ajouter les enfants à la stack
            stack.extend(children);

            // Compter si avait une valeur
            if has_val {
                count += 1;
            }

            // Supprimer le huge_childs si présent
            if let Some(huge_idx) = huge_child_idx {
                self.child_list.remove(huge_idx as usize);
            }

            // Supprimer le node de la slab
            self.map.remove(node_idx as usize);
        }

        count
    }

    /// Si le node a exactement 1 enfant et pas de valeur, absorbe l'enfant
    fn try_recompress(&mut self, node_idx: u32) {
        let node = self.get_node(node_idx);
        if node.val.is_some() {
            return;
        }

        let Some((child_radix, child_idx)) = node.childs.get_single_child() else {
            return;
        };

        // Absorber l'enfant: compression = current + radix + child.compression
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
        // Sinon c'est dans huge_childs
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
