pub use alloc::collections::*;

use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

struct FnvHasher(u64);

impl FnvHasher {
    fn new() -> Self {
        FnvHasher(axhal::misc::random() as u64)
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}

fn fnv_hash<K: Hash>(key: &K) -> u64 {
    let mut h = FnvHasher::new();
    key.hash(&mut h);
    h.finish()
}

const INITIAL_BUCKETS: usize = 16;

/// 基于分离链表法的 HashMap，不依赖任何外部 crate。
pub struct HashMap<K, V> {
    buckets: Vec<Vec<(K, V)>>,
    len: usize,
}

impl<K: Hash + Eq, V> HashMap<K, V> {
    pub fn new() -> Self {
        let buckets = (0..INITIAL_BUCKETS).map(|_| Vec::new()).collect();
        HashMap { buckets, len: 0 }
    }

    fn bucket_idx(&self, key: &K) -> usize {
        fnv_hash(key) as usize % self.buckets.len()
    }

    /// 插入键值对，若 key 已存在则覆盖并返回旧值。
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.len >= self.buckets.len() * 3 / 4 {
            self.grow();
        }
        let idx = self.bucket_idx(&key);
        for (k, v) in self.buckets[idx].iter_mut() {
            if k == &key {
                return Some(core::mem::replace(v, value));
            }
        }
        self.buckets[idx].push((key, value));
        self.len += 1;
        None
    }

    /// 遍历所有键值对。
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.buckets
            .iter()
            .flat_map(|b| b.iter().map(|(k, v)| (k, v)))
    }

    /// 扩容：桶数翻倍后重新散列。
    fn grow(&mut self) {
        let new_cap = self.buckets.len() * 2;
        let mut new_buckets: Vec<Vec<(K, V)>> = (0..new_cap).map(|_| Vec::new()).collect();
        for bucket in self.buckets.drain(..) {
            for (k, v) in bucket {
                let idx = fnv_hash(&k) as usize % new_cap;
                new_buckets[idx].push((k, v));
            }
        }
        self.buckets = new_buckets;
    }
}
