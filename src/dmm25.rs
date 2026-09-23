//! A faithful implementation of Duan, Mao, Mao, Shu, Yin, "Breaking the Sorting Barrier for
//! Directed Single-Source Shortest Paths" (arXiv 2504.17033), Algorithms 1–3:
//! * Sec. 2 constant-degree transformation (every vertex becomes a zero-weight cycle);
//! * Algorithm 1 FindPivots (k Bellman–Ford rounds, pivots = roots of trees of size >= k);
//! * Algorithm 2 BaseCase (Dijkstra from a single source until k+1 vertices);
//! * Algorithm 3 BMSSP with the Lemma 3.3 block structure (Insert / BatchPrepend / Pull);
//! * k = ⌊lg^{1/3} n⌋, t = ⌊lg^{2/3} n⌋, L = ⌈lg n / t⌉, M = 2^{(l−1)t}.
//!
//! Two errata recorded by the C-HD team (B1 §6, "FIX-STALE" and "FIX-EMPTY") are applied:
//! after a sub-call its U_i is deleted from D, and B' := B when D ends empty.
//! Ties are broken with the (len, hops, v, e, version) label of `common::Label`.

use crate::common::*;
use crate::graph::Graph;
use std::collections::BinaryHeap;
use std::time::Instant;

struct Gadget {
    n: usize,
    st: Vec<u32>,
    head: Vec<u32>,
    w: Vec<f64>,
    tail: Vec<u32>,
    base: Vec<u32>,
}

fn gadget(g: &Graph) -> Gadget {
    let n = g.n;
    let mut deg = vec![0usize; n];
    for u in 0..n {
        for e in &g.edges[u] {
            if e.to != u {
                deg[u] += 1;
                deg[e.to] += 1;
            }
        }
    }
    let mut base = vec![0u32; n];
    let mut nv = 0usize;
    for v in 0..n {
        base[v] = nv as u32;
        nv += deg[v].max(1);
    }
    let mut fill = vec![0u32; n];
    let mut edges: Vec<(u32, u32, f64)> = Vec::new();
    for v in 0..n {
        let c = deg[v].max(1);
        if c >= 2 {
            for i in 0..c {
                edges.push((base[v] + i as u32, base[v] + ((i + 1) % c) as u32, 0.0));
            }
        }
    }
    for u in 0..n {
        for e in &g.edges[u] {
            if e.to == u {
                continue;
            }
            let a = base[u] + fill[u];
            fill[u] += 1;
            let b = base[e.to] + fill[e.to];
            fill[e.to] += 1;
            edges.push((a, b, e.weight));
        }
    }
    let mut st = vec![0u32; nv + 1];
    for &(a, _, _) in &edges {
        st[a as usize + 1] += 1;
    }
    for i in 0..nv {
        st[i + 1] += st[i];
    }
    let mut pos = st.clone();
    let mut head = vec![0u32; edges.len()];
    let mut w = vec![0f64; edges.len()];
    let mut tail = vec![0u32; edges.len()];
    for &(a, b, wt) in &edges {
        let p = pos[a as usize] as usize;
        pos[a as usize] += 1;
        head[p] = b;
        w[p] = wt;
        tail[p] = a;
    }
    Gadget { n: nv, st, head, w, tail, base }
}

struct Dmm<'a> {
    g: &'a Gadget,
    d: Vec<Label>,
    ver: Vec<u32>,
    ds: DsState,
    k: usize,
    t: u32,
    ids: Ids,
    wst: Vec<u32>,
    rst: Vec<u32>,
    umark: Vec<u32>,
    kmark: Vec<u32>,
    fch: Vec<u32>,
    fnx: Vec<u32>,
    fst: Vec<u32>,
    haspar: Vec<u32>,
    stats: Stats,
}

impl<'a> Dmm<'a> {
    #[inline]
    fn cand(&self, u: u32, e: usize) -> Label {
        let du = &self.d[u as usize];
        Label { len: du.len + self.g.w[e], hops: du.hops.wrapping_add(1), v: self.g.head[e], e: e as u32 + 1, a: self.ver[u as usize] }
    }

    #[inline]
    fn relax(&mut self, c: &Label, b: &Label) -> bool {
        self.stats.relax_calls += 1;
        let v = c.v as usize;
        if !(c.le(&self.d[v]) && c.lt(b)) {
            return false;
        }
        if c.lt(&self.d[v]) {
            self.d[v] = *c;
            self.ver[v] += 1;
        }
        true
    }

    fn pow2(&self, sh: u64) -> u64 {
        if sh >= 40 {
            1u64 << 40
        } else {
            1u64 << sh
        }
    }

    /// Algorithm 1.
    fn find_pivots(&mut self, b: Label, s: &[u32]) -> (Vec<u32>, Vec<u32>) {
        let id = self.ids.next();
        let mut w: Vec<u32> = s.to_vec();
        for &x in s {
            self.wst[x as usize] = id;
        }
        let mut frontier: Vec<u32> = s.to_vec();
        for _ in 0..self.k {
            let rid = self.ids.next();
            let mut next = Vec::new();
            for &u in &frontier {
                for e in self.g.st[u as usize] as usize..self.g.st[u as usize + 1] as usize {
                    let c = self.cand(u, e);
                    if self.relax(&c, &b) {
                        let v = c.v as usize;
                        if self.rst[v] != rid {
                            self.rst[v] = rid;
                            next.push(c.v);
                        }
                        if self.wst[v] != id {
                            self.wst[v] = id;
                            w.push(c.v);
                        }
                    }
                }
            }
            if w.len() > self.k * s.len() {
                return (s.to_vec(), w);
            }
            frontier = next;
        }
        // F = tight edges inside W; P = roots in S of trees with >= k vertices
        let fid = self.ids.next();
        for &v in &w {
            let lab = self.d[v as usize];
            if lab.e == 0 {
                continue;
            }
            let e = (lab.e - 1) as usize;
            let p = self.g.tail[e];
            if self.wst[p as usize] == id && self.cand(p, e) == lab {
                if self.fst[p as usize] != fid {
                    self.fst[p as usize] = fid;
                    self.fch[p as usize] = NONE;
                }
                self.fnx[v as usize] = self.fch[p as usize];
                self.fch[p as usize] = v;
                self.haspar[v as usize] = fid;
            }
        }
        let mut piv = Vec::new();
        let mut stack = Vec::new();
        for &x in s {
            if self.haspar[x as usize] == fid {
                continue;
            }
            let mut size = 0usize;
            stack.clear();
            stack.push(x);
            while let Some(u) = stack.pop() {
                size += 1;
                if size >= self.k {
                    break;
                }
                if self.fst[u as usize] == fid {
                    let mut c = self.fch[u as usize];
                    while c != NONE {
                        stack.push(c);
                        c = self.fnx[c as usize];
                    }
                }
            }
            if size >= self.k {
                piv.push(x);
            }
        }
        (piv, w)
    }

    /// Algorithm 2 (S is a singleton).
    fn base_case(&mut self, b: Label, s: Vec<u32>) -> (Label, Vec<u32>) {
        self.stats.base_calls += 1;
        debug_assert_eq!(s.len(), 1);
        let bid = self.ids.next();
        let mut heap = BinaryHeap::new();
        heap.push(HeapItem(self.d[s[0] as usize], s[0]));
        let mut u0: Vec<u32> = Vec::new();
        while u0.len() < self.k + 1 {
            let Some(HeapItem(lab, u)) = heap.pop() else { break };
            if lab != self.d[u as usize] || self.umark[u as usize] == bid {
                continue;
            }
            self.umark[u as usize] = bid;
            u0.push(u);
            for e in self.g.st[u as usize] as usize..self.g.st[u as usize + 1] as usize {
                let c = self.cand(u, e);
                if self.relax(&c, &b) {
                    heap.push(HeapItem(self.d[c.v as usize], c.v));
                }
            }
        }
        if u0.len() <= self.k {
            return (b, u0);
        }
        let mut bp = Label { len: f64::NEG_INFINITY, hops: 0, v: 0, e: 0, a: u32::MAX };
        for &u in &u0 {
            let du = self.d[u as usize];
            if bp.lt(&du) {
                bp = du;
            }
        }
        let u: Vec<u32> = u0.into_iter().filter(|&v| self.d[v as usize].lt(&bp)).collect();
        (bp, u)
    }

    /// Algorithm 3.
    fn bmssp(&mut self, l: u32, b: Label, s: Vec<u32>) -> (Label, Vec<u32>) {
        self.stats.calls += 1;
        if l == 0 {
            return self.base_case(b, s);
        }
        let (p, w) = self.find_pivots(b, &s);
        let m = self.pow2((l as u64 - 1) * self.t as u64) as usize;
        let cap = (self.k as u64).saturating_mul(self.pow2(l as u64 * self.t as u64));
        let mut dd = BlockDs::new(m, b);
        let mut bp = b;
        for &x in &p {
            let dx = self.d[x as usize];
            dd.insert(&mut self.ds, x, dx);
            bp = bp.min(dx);
        }
        let call = self.ids.next();
        let mut u_all: Vec<u32> = Vec::new();
        let mut kb: Vec<(u32, Label)> = Vec::new();
        while (u_all.len() as u64) < cap && !dd.is_empty(&self.ds) {
            let (si, bi) = dd.pull(&mut self.ds);
            let (bpi, ui) = self.bmssp(l - 1, bi, si.clone());
            for &u in &ui {
                self.ds.delete(u); // FIX-STALE
                self.umark[u as usize] = call;
            }
            kb.clear();
            let kid = self.ids.next();
            for &u in &ui {
                for e in self.g.st[u as usize] as usize..self.g.st[u as usize + 1] as usize {
                    let c = self.cand(u, e);
                    if self.relax(&c, &b) {
                        let v = c.v;
                        if !c.lt(&bi) {
                            let dv = self.d[v as usize];
                            dd.insert(&mut self.ds, v, dv);
                        } else if !c.lt(&bpi) && self.kmark[v as usize] != kid {
                            self.kmark[v as usize] = kid;
                            kb.push((v, c));
                        }
                    }
                }
            }
            for &x in &si {
                let dx = self.d[x as usize];
                if bpi.le(&dx) && dx.lt(&bi) && self.kmark[x as usize] != kid {
                    self.kmark[x as usize] = kid;
                    kb.push((x, dx));
                }
            }
            for item in kb.iter_mut() {
                item.1 = self.d[item.0 as usize]; // current label (it may have decreased since)
            }
            dd.batch_prepend(&mut self.ds, &kb, bi);
            bp = bpi;
            u_all.extend_from_slice(&ui);
        }
        if dd.is_empty(&self.ds) {
            bp = b; // FIX-EMPTY
        }
        dd.clear(&mut self.ds); // DMM+25 discards the level's D; its keys are re-found by the parent
        for x in w {
            if self.umark[x as usize] != call && self.d[x as usize].lt(&bp) {
                self.umark[x as usize] = call;
                u_all.push(x);
            }
        }
        (bp, u_all)
    }
}

pub fn dmm25_sssp(g: &Graph, s: usize) -> (Vec<f64>, Stats) {
    let t0 = Instant::now();
    let gg = gadget(g);
    let pre = t0.elapsed().as_secs_f64();
    let t1 = Instant::now();
    let nv = gg.n;
    let lg = (nv.max(2) as f64).log2();
    let k = (lg.powf(1.0 / 3.0).floor() as usize).max(1);
    let t = (lg.powf(2.0 / 3.0).floor() as u32).max(1);
    let levels = (lg / t as f64).ceil() as u32;
    let mut c = Dmm {
        g: &gg,
        d: vec![Label::INF; nv],
        ver: vec![0; nv],
        ds: DsState::new(nv),
        k,
        t,
        ids: Ids(0),
        wst: vec![0; nv],
        rst: vec![0; nv],
        umark: vec![0; nv],
        kmark: vec![0; nv],
        fch: vec![NONE; nv],
        fnx: vec![NONE; nv],
        fst: vec![0; nv],
        haspar: vec![0; nv],
        stats: Stats::default(),
    };
    let src = gg.base[s];
    c.d[src as usize] = Label::source(src);
    let _ = c.bmssp(levels, Label::INF, vec![src]);
    let dist: Vec<f64> = (0..g.n).map(|v| c.d[gg.base[v] as usize].len).collect();
    let mut stats = c.stats;
    stats.n_reduced = nv;
    stats.m_reduced = gg.head.len();
    stats.t = t;
    stats.k = k as u32;
    stats.levels = levels;
    stats.preprocess_secs = pre;
    stats.core_secs = t1.elapsed().as_secs_f64();
    (dist, stats)
}
