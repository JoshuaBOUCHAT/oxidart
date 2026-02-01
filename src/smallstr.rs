use std::{
    cmp::min,
    ops::{BitAnd, Index, IndexMut},
};

#[derive(Clone, Default)]
pub struct SmallStr {
    pub size: u8,
    pub data: [u8; 19],
}
impl SmallStr {
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..(self.size as usize)]
    }
    pub fn as_str(&self) -> &str {
        str::from_utf8(self.as_slice()).expect("data had to be ascii should not have err")
    }
    pub fn common_at(&self, key: &[u8], cursor: usize) -> usize {
        let max = (self.size as usize).min(key.len() - cursor);
        if max == 0 {
            return 0;
        }
        for i in 0..max {
            if self[i] != key[i] {
                return i;
            }
        }
        max
    }
    pub fn len(&self) -> usize {
        self.size as usize
    }
}
impl From<&[u8]> for SmallStr {
    fn from(value: &[u8]) -> Self {
        let size = min(value.len(), 19);
        let mut data = [0u8; 19];
        for i in 0..size {
            data[i] = value[i];
        }
        Self {
            size: size as u8,
            data,
        }
    }
}

impl Index<usize> for SmallStr {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}
impl IndexMut<usize> for SmallStr {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

impl BitAnd for &SmallStr {
    type Output = usize;
    fn bitand(self, rhs: Self) -> Self::Output {
        let max = self.size.min(rhs.size) as usize;
        for i in 0..max {
            if self[i] != rhs[i] {
                return i;
            }
        }
        max
    }
}
