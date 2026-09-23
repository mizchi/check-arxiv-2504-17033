//! Shared building blocks for the faithful BMSSP-family implementations
//! (`dmm25` = arXiv 2504.17033, `chd` = C-HD of spicylemonade/c-hd-proof).
//!
//! * `Label`: the 5-tuple label (len, hops, v, e, a) of the C-HD / B1 papers. It realises a strict
//!   total order on walks, which replaces the "all path lengths are distinct" assumption of
//!   DMM+25 (Sec. 2 of arXiv 2504.17033).
//! * `BlockDs` / `DsState`: the block-based partial-sorting structure (DMM+25 Lemma 3.3 /
//!   C-HD DS'): unsorted blocks with increasing value intervals, binary search over block
//!   separators, lazy front splits, Pull by selection, Merge by block splicing, stale entries
//!   discarded lazily via per-key stamps.
//!
//! Exactness note: labels add weights with f64 `+`. The benchmarks use integer weights
//! (< 2^32), so every sum is exact and the comparison-addition model is simulated faithfully.

use std::cmp::Ordering;

pub const NONE: u32 = u32::MAX;

/// Number of weight/label comparisons (only counted with `--features count-cmp`).
#[cfg(feature = "count-cmp")]
pub static CMP_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[inline(always)]
pub fn count_cmp() {
    #[cfg(feature = "count-cmp")]
    CMP_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Label {
    pub len: f64,
    pub hops: u32,
    pub v: u32,
    /// last edge id + 1 (0 = NOEDGE, the smallest)
    pub e: u32,
    /// version of the tail at write time (larger version = smaller label)
    pub a: u32,
}

impl Label {
    pub const INF: Label = Label { len: f64::INFINITY, hops: u32::MAX, v: u32::MAX, e: u32::MAX, a: 0 };

    #[inline]
    pub fn source(v: u32) -> Label {
        Label { len: 0.0, hops: 0, v, e: 0, a: 0 }
    }

    #[inline]
    pub fn cmp(&self, o: &Label) -> Ordering {
        count_cmp();
        if self.len != o.len {
            return if self.len < o.len { Ordering::Less } else { Ordering::Greater };
        }
        self.hops
            .cmp(&o.hops)
            .then(self.v.cmp(&o.v))
            .then(self.e.cmp(&o.e))
            .then(o.a.cmp(&self.a))
    }

    #[inline]
    pub fn lt(&self, o: &Label) -> bool {
        self.cmp(o) == Ordering::Less
    }

    #[inline]
    pub fn le(&self, o: &Label) -> bool {
        self.cmp(o) != Ordering::Greater
    }

    #[inline]
    pub fn min(self, o: Label) -> Label {
        if o.lt(&self) {
            o
        } else {
            self
        }
    }
}

/// Wrapper giving `Label` a total `Ord` (min-heap via `Reverse`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeapItem(pub Label, pub u32);
impl Eq for HeapItem {}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for HeapItem {
    fn cmp(&self, o: &Self) -> Ordering {
        // reversed: BinaryHeap is a max-heap
        o.0.cmp(&self.0)
    }
}

/// Global per-key bookkeeping shared by every block structure of one run
/// (one live entry per key, "live[v]" of DS').
pub struct DsState {
    stamp: Vec<u32>,
    val: Vec<Label>,
    alive: Vec<bool>,
    counter: u32,
}

impl DsState {
    pub fn new(n: usize) -> Self {
        DsState { stamp: vec![0; n], val: vec![Label::INF; n], alive: vec![false; n], counter: 0 }
    }

    #[inline]
    fn live(&self, e: &Entry) -> bool {
        self.stamp[e.v as usize] == e.st
    }

    /// Returns the stamp for a new live entry, or None if an entry with value <= c is live.
    #[inline]
    fn try_insert(&mut self, v: u32, c: Label) -> Option<u32> {
        let vi = v as usize;
        if self.alive[vi] && self.val[vi].le(&c) {
            return None;
        }
        self.counter += 1;
        self.stamp[vi] = self.counter;
        self.val[vi] = c;
        self.alive[vi] = true;
        Some(self.counter)
    }

    #[inline]
    pub fn delete(&mut self, v: u32) {
        let vi = v as usize;
        if self.alive[vi] {
            self.alive[vi] = false;
            self.counter += 1;
            self.stamp[vi] = self.counter;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub key: Label,
    pub v: u32,
    pub st: u32,
}

pub struct Block {
    /// exclusive upper bound of the block's value interval
    pub ub: Label,
    pub items: Vec<Entry>,
}

/// Blocks are stored back-to-front: `blocks.last()` is the FRONT (smallest values).
pub struct BlockDs {
    pub blocks: Vec<Block>,
    pub m: usize,
    pub bound: Label,
}

impl BlockDs {
    pub fn new(m: usize, bound: Label) -> Self {
        BlockDs { blocks: Vec::new(), m: m.max(1), bound }
    }

    pub fn insert(&mut self, st: &mut DsState, v: u32, c: Label) {
        let Some(s) = st.try_insert(v, c) else { return };
        let e = Entry { key: c, v, st: s };
        if self.blocks.is_empty() {
            self.blocks.push(Block { ub: self.bound, items: vec![e] });
            return;
        }
        // ub decreases with the index; the target is the frontmost block with c < ub
        let p = self.blocks.partition_point(|b| c.lt(&b.ub));
        let idx = if p == 0 { 0 } else { p - 1 };
        self.blocks[idx].items.push(e);
    }

    /// Prepends a batch of keys whose values are all < `ub` <= every value in the structure
    /// (DMM+25 BatchPrepend).
    pub fn batch_prepend(&mut self, st: &mut DsState, items: &[(u32, Label)], ub: Label) {
        let mut blk = Vec::with_capacity(items.len());
        for &(v, c) in items {
            if let Some(s) = st.try_insert(v, c) {
                blk.push(Entry { key: c, v, st: s });
            }
        }
        if !blk.is_empty() {
            self.blocks.push(Block { ub, items: blk });
        }
    }

    /// Splices `child`'s blocks in front (all child values are below all values left here).
    pub fn merge(&mut self, child: BlockDs) {
        self.blocks.extend(child.blocks);
    }

    /// Discards stale front entries and splits the front block lazily until it is small.
    fn prepare_front(&mut self, st: &DsState) {
        let lim = 2 * self.m;
        loop {
            let Some(f) = self.blocks.last_mut() else { return };
            f.items.retain(|e| st.live(e));
            if f.items.is_empty() {
                self.blocks.pop();
                continue;
            }
            if f.items.len() > lim {
                let mid = f.items.len() / 2;
                f.items.select_nth_unstable_by(mid, |a, b| a.key.cmp(&b.key));
                let upper = f.items.split_off(mid);
                let pivot = upper[0].key;
                let lower = std::mem::replace(&mut f.items, upper);
                self.blocks.push(Block { ub: pivot, items: lower });
                continue;
            }
            return;
        }
    }

    pub fn is_empty(&mut self, st: &DsState) -> bool {
        self.prepare_front(st);
        self.blocks.is_empty()
    }

    /// Pull: returns the (at most) M smallest live keys and a separating bound B_i.
    pub fn pull(&mut self, st: &mut DsState) -> (Vec<u32>, Label) {
        let mut coll: Vec<Entry> = Vec::new();
        let mut last_ub = self.bound;
        loop {
            self.prepare_front(st);
            match self.blocks.pop() {
                None => break,
                Some(b) => {
                    last_ub = b.ub;
                    coll.extend(b.items);
                    if coll.len() > self.m {
                        break;
                    }
                }
            }
        }
        let bi;
        if coll.len() <= self.m {
            bi = self.bound;
            debug_assert!(self.blocks.is_empty());
        } else {
            coll.select_nth_unstable_by(self.m, |a, b| a.key.cmp(&b.key));
            let rest = coll.split_off(self.m);
            bi = rest.iter().fold(Label::INF, |acc, e| acc.min(e.key));
            self.blocks.push(Block { ub: last_ub, items: rest });
        }
        let keys: Vec<u32> = coll.iter().map(|e| e.v).collect();
        for &v in &keys {
            st.delete(v);
        }
        (keys, bi)
    }

    /// Destroys the structure, releasing its live keys (a discarded D must not keep keys live).
    pub fn clear(self, st: &mut DsState) {
        for b in self.blocks {
            for e in b.items {
                if st.live(&e) {
                    st.delete(e.v);
                }
            }
        }
    }

    /// Live (key, value) pairs; used only for tests/debugging.
    pub fn live_items(&self, st: &DsState) -> Vec<(u32, Label)> {
        self.blocks.iter().flat_map(|b| b.items.iter()).filter(|e| st.live(e)).map(|e| (e.v, e.key)).collect()
    }
}

/// Monotone id source for O(1) "clearing" of mark arrays.
pub struct Ids(pub u32);
impl Ids {
    #[inline]
    pub fn next(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
}

/// Counters reported by the new implementations.
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub n_reduced: usize,
    pub m_reduced: usize,
    pub t: u32,
    pub k: u32,
    pub levels: u32,
    pub relax_calls: u64,
    pub edge_deletions: u64,
    pub calls: u64,
    pub base_calls: u64,
    pub preprocess_secs: f64,
    pub core_secs: f64,
}
