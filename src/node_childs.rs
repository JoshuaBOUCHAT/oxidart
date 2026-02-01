use arrayvec::ArrayVec;

pub(crate) const CHILDS_SIZE: usize = 10;
const ASCII_MAX_CHAR: usize = 127;
pub(crate) const HUGE_CHILDS_SIZE: usize = ASCII_MAX_CHAR - CHILDS_SIZE;

#[repr(C, align(64))]
pub(crate) struct Childs {
    idxs: ArrayVec<u32, CHILDS_SIZE>,
    radixs: ArrayVec<u8, CHILDS_SIZE>,
    maybe_next_childs_idx: u32,
}
pub(crate) trait ChildAble {
    fn find(&self, radix: u8) -> Option<u32>;
    fn push(&mut self, radix: u8, idx: u32);
    fn remove(&mut self, radix: u8) -> Option<u32>;
    fn is_empty(&self) -> bool;
    fn iter(&self) -> impl Iterator<Item = (u8, u32)>;
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
impl ChildAble for Childs {
    fn find(&self, radix: u8) -> Option<u32> {
        self.radixs
            .iter()
            .position(|&c| c == radix)
            .map(|i| self.idxs[i])
    }

    fn push(&mut self, radix: u8, idx: u32) {
        assert!(!self.is_full());
        self.idxs.push(idx);
        self.radixs.push(radix);
    }

    fn remove(&mut self, radix: u8) -> Option<u32> {
        let pos = self.radixs.iter().position(|&c| c == radix)?;
        self.radixs.swap_remove(pos);
        Some(self.idxs.swap_remove(pos))
    }

    fn is_empty(&self) -> bool {
        self.idxs.is_empty()
    }

    fn iter(&self) -> impl Iterator<Item = (u8, u32)> {
        self.radixs.iter().copied().zip(self.idxs.iter().copied())
    }
}

impl Childs {
    pub(crate) fn get_next_idx(&self) -> Option<u32> {
        if self.maybe_next_childs_idx == u32::MAX {
            None
        } else {
            Some(self.maybe_next_childs_idx)
        }
    }
    pub(crate) fn is_full(&self) -> bool {
        self.idxs.is_full()
    }
    pub(crate) fn set_new_childs(&mut self, idx: u32) {
        assert!(self.maybe_next_childs_idx == u32::MAX);
        self.maybe_next_childs_idx = idx
    }
    /// Retourne (radix, idx) si exactement 1 enfant et pas de huge_childs
    pub(crate) fn get_single_child(&self) -> Option<(u8, u32)> {
        if self.idxs.len() == 1 && self.maybe_next_childs_idx == u32::MAX {
            Some((self.radixs[0], self.idxs[0]))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct HugeChildRegistry {
    radix: u8,
    idx: u32,
}

#[repr(align(64))]
#[derive(Default)]
pub(crate) struct HugeChilds {
    entries: ArrayVec<HugeChildRegistry, HUGE_CHILDS_SIZE>,
}

impl HugeChilds {
    pub(crate) fn new(radix: u8, idx: u32) -> Self {
        let mut entries = ArrayVec::new_const();
        entries.push(HugeChildRegistry { radix, idx });
        Self { entries }
    }
}

impl ChildAble for HugeChilds {
    fn find(&self, radix: u8) -> Option<u32> {
        self.entries
            .iter()
            .find(|e| e.radix == radix)
            .map(|e| e.idx)
    }

    fn push(&mut self, radix: u8, idx: u32) {
        self.entries.push(HugeChildRegistry { radix, idx });
    }

    fn remove(&mut self, radix: u8) -> Option<u32> {
        let pos = self.entries.iter().position(|e| e.radix == radix)?;
        Some(self.entries.swap_remove(pos).idx)
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn iter(&self) -> impl Iterator<Item = (u8, u32)> {
        self.entries.iter().map(|e| (e.radix, e.idx))
    }
}
