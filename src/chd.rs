//! C-HD (spicylemonade/c-hd-proof, paper/PAPER.md v2.0), re-implemented in Rust from the paper text.
//!
//! The Lean artefact is a deep-embedded RAM program over `ℝ≥0` and is not executable, so this is
//! an independent implementation of the paper's algorithm:
//! * §2 preprocessing P1–P5 (keep-set, dedup, out-degree chains with δ = max(3, ⌈2m/n⌉),
//!   static CSR ranges sorted by (w, head) plus a mutable live list for FindPivots-HD);
//! * §3.2 the core = B1 PCv2.3 BM.1–BM.31 / BC.1–BC.9 with M1 (pointer scans over the static
//!   range), M2 (FH.9 permanent edge deletion), M3 (FindPivots-HD), M4 (block DS), M6 (no BM.22);
//! * §3.3 FindPivots-HD FH.1–FH.26 with an unsorted-array search heap, MakePivots MP.1–MP.10 and
//!   tree Partition PT.1–PT.9;
//! * §6 parameters: t = max(t0, ⌈(lg N / d)^{2/3}⌉), k = ⌈√t⌉, M_l = t·2^{(l−1)t},
//!   Λ_l = t³·2^{lt}, L = ⌊lg N / t⌋ + 1; the paper fixes t0 = 16.
//!
//! Not reproduced: the formal program's dispatcher, which runs Bellman–Ford when log₂ n < 16 or
//! ⌈m/n⌉ > F(n). `formal_branch` reports which branch it would take.

use crate::common::*;
use crate::graph::Graph;
use std::collections::BinaryHeap;
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub struct ChdParams {
    pub t0: u32,
}

impl Default for ChdParams {
    fn default() -> Self {
        ChdParams { t0: 16 }
    }
}

/// F(n) = ⌊⌊log₂ n⌋^{3/4}⌋ and the branch the formal `chdProgram` dispatcher would take.
pub fn formal_branch(n: usize, m: usize) -> &'static str {
    let lg = (usize::BITS - 1 - n.max(1).leading_zeros()) as f64;
    let f = (lg.powi(3).sqrt().floor().sqrt()).floor() as usize;
    if lg < 16.0 || m.div_ceil(n.max(1)) > f {
        "Bellman-Ford"
    } else {
        "C-HD"
    }
}

/// Graph after §2 preprocessing.
pub struct Reduced {
    pub n: usize,
    pub st: Vec<u32>,
    pub head: Vec<u32>,
    pub w: Vec<f64>,
    /// original vertex -> first chunk (NONE if not kept)
    pub off: Vec<u32>,
    pub src: u32,
    pub delta: usize,
}

pub fn preprocess(g: &Graph, s: usize) -> Reduced {
    let n = g.n;
    let m_in = g.m();
    // P1 keep-set
    let mut keep = vec![false; n];
    keep[s] = true;
    for u in 0..n {
        for e in &g.edges[u] {
            if e.to != u {
                keep[e.to] = true;
            }
        }
    }
    // P2 dedup: first minimum-weight edge per (u, v), no self-loops
    let mut lists: Vec<Vec<(f64, u32)>> = vec![Vec::new(); n];
    for u in 0..n {
        if !keep[u] {
            continue;
        }
        let mut l: Vec<(u32, f64, u32)> = g.edges[u]
            .iter()
            .enumerate()
            .filter(|(_, e)| e.to != u)
            .map(|(i, e)| (e.to as u32, e.weight, i as u32))
            .collect();
        l.sort_by(|a, b| {
            count_cmp();
            a.0.cmp(&b.0).then(a.1.partial_cmp(&b.1).unwrap()).then(a.2.cmp(&b.2))
        });
        l.dedup_by(|b, a| a.0 == b.0);
        lists[u] = l.into_iter().map(|(v, w, _)| (w, v)).collect();
    }
    // P3 out-degree chains
    let delta = 3usize.max((2 * m_in).div_ceil(n.max(1)));
    let mut off = vec![NONE; n];
    let mut chunks = vec![0usize; n];
    let mut nv = 0usize;
    for u in 0..n {
        if keep[u] {
            let du = lists[u].len();
            chunks[u] = 1.max(du.div_ceil(delta - 1));
            off[u] = nv as u32;
            nv += chunks[u];
        }
    }
    // P4 CSR ranges sorted by (w, head)
    let mut st = Vec::with_capacity(nv + 1);
    let mut head = Vec::new();
    let mut w = Vec::new();
    st.push(0u32);
    let mut tmp: Vec<(f64, u32)> = Vec::new();
    for u in 0..n {
        if !keep[u] {
            continue;
        }
        let mut l: Vec<(f64, u32)> = lists[u].iter().map(|&(wt, v)| (wt, off[v as usize])).collect();
        l.sort_by(|a, b| {
            count_cmp();
            a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1))
        });
        let c = chunks[u];
        for q in 0..c {
            tmp.clear();
            let lo = q * (delta - 1);
            let hi = ((q + 1) * (delta - 1)).min(l.len());
            if lo < hi {
                tmp.extend_from_slice(&l[lo..hi]);
            }
            if q + 1 < c {
                tmp.push((0.0, off[u] + q as u32 + 1));
            }
            tmp.sort_by(|a, b| {
                count_cmp();
                a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1))
            });
            for &(wt, h) in &tmp {
                head.push(h);
                w.push(wt);
            }
            st.push(head.len() as u32);
        }
    }
    let src = off[s];
    Reduced { n: nv, st, head, w, off, src, delta }
}

struct Tree {
    verts: Vec<u32>,
    edges: Vec<(u32, u32)>,
}

struct PmRec {
    owner: u32,
    j: u32,
    next: u32,
}

struct Chd<'a> {
    g: &'a Reduced,
    d: Vec<Label>,
    ver: Vec<u32>,
    ptr: Vec<u32>,
    hd: Vec<u32>,
    nxt: Vec<u32>,
    ds: DsState,
    t: u32,
    k: usize,
    ids: Ids,
    // Pmem stacks
    pm_head: Vec<u32>,
    pm: Vec<PmRec>,
    pm_free: Vec<u32>,
    // FindPivots marks
    fmark: Vec<u32>,
    ftree: Vec<u32>,
    kst: Vec<u32>,
    kpar: Vec<u32>,
    valst: Vec<u32>,
    hst: Vec<u32>,
    wst: Vec<u32>,
    ins: Vec<u32>,
    inq: Vec<u32>,
    asg: Vec<u32>,
    loc: Vec<u32>,
    // BM marks
    umark: Vec<u32>,
    sist: Vec<u32>,
    stats: Stats,
}

impl<'a> Chd<'a> {
    #[inline]
    fn cand(&self, u: u32, e: usize) -> Label {
        let du = &self.d[u as usize];
        Label { len: du.len + self.g.w[e], hops: du.hops.wrapping_add(1), v: self.g.head[e], e: e as u32 + 1, a: self.ver[u as usize] }
    }

    /// RX.1–RX.9 (no suppression hook: C-HD has no inner search).
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

    fn lambda(&self, l: u32) -> u64 {
        let t = self.t as u64;
        let sh = (l as u64) * t;
        let p = if sh >= 40 { 1u64 << 40 } else { 1u64 << sh };
        (t * t * t).saturating_mul(p)
    }

    fn mpull(&self, l: u32) -> usize {
        let t = self.t as u64;
        let sh = (l as u64 - 1) * t;
        let p = if sh >= 40 { 1u64 << 40 } else { 1u64 << sh };
        t.saturating_mul(p) as usize
    }

    // ---- Pmem (per-vertex stacks of (owner, group)) ----
    fn pm_push(&mut self, v: u32, owner: u32, j: u32) {
        let rec = PmRec { owner, j, next: self.pm_head[v as usize] };
        let idx = if let Some(i) = self.pm_free.pop() {
            self.pm[i as usize] = rec;
            i
        } else {
            self.pm.push(rec);
            (self.pm.len() - 1) as u32
        };
        self.pm_head[v as usize] = idx;
    }
    #[inline]
    fn pm_get(&self, v: u32, owner: u32) -> Option<u32> {
        let h = self.pm_head[v as usize];
        if h != NONE && self.pm[h as usize].owner == owner {
            Some(self.pm[h as usize].j)
        } else {
            None
        }
    }
    fn pm_pop(&mut self, v: u32, owner: u32) {
        let h = self.pm_head[v as usize];
        if h != NONE && self.pm[h as usize].owner == owner {
            self.pm_head[v as usize] = self.pm[h as usize].next;
            self.pm_free.push(h);
        }
    }

    // ---- FindPivots-HD (FH.1–FH.26) ----
    fn find_pivots(&mut self, b: Label, s: &[u32]) -> (Vec<Vec<u32>>, Vec<u32>) {
        let k = self.k;
        let mut lx = b;
        for &x in s {
            lx = lx.min(self.d[x as usize]);
        }
        let id = self.ids.next();
        let mut trees: Vec<Tree> = Vec::new();
        let mut w_out: Vec<u32> = Vec::new();
        let mut q: Vec<u32> = Vec::new();
        let mut h: Vec<u32> = Vec::new();
        let mut kk: Vec<u32> = Vec::new();
        let mut val: Vec<u32> = Vec::new();
        for &x in s {
            let xi = x as usize;
            if self.fmark[xi] == id {
                continue;
            }
            let sid = self.ids.next();
            h.clear();
            kk.clear();
            val.clear();
            h.push(x);
            self.hst[xi] = sid;
            kk.push(x);
            self.kst[xi] = sid;
            self.kpar[xi] = NONE;
            self.valst[xi] = sid;
            val.push(x);
            let mut contact = false;
            'search: while !h.is_empty() && kk.len() < k {
                // ExtractMin on the unsorted array
                let mut bi = 0;
                for i in 1..h.len() {
                    if self.d[h[i] as usize].lt(&self.d[h[bi] as usize]) {
                        bi = i;
                    }
                }
                let u = h.swap_remove(bi);
                self.hst[u as usize] = 0;
                let mut prev = NONE;
                let mut e = self.hd[u as usize];
                while e != NONE {
                    let next = self.nxt[e as usize];
                    let c = self.cand(u, e as usize);
                    if !c.lt(&b) {
                        break; // FH.8 range stop (prefix fact)
                    }
                    let v = c.v;
                    let vi = v as usize;
                    if self.d[vi].lt(&lx) {
                        // FH.9 permanent deletion
                        if prev == NONE {
                            self.hd[u as usize] = next;
                        } else {
                            self.nxt[prev as usize] = next;
                        }
                        self.stats.edge_deletions += 1;
                        e = next;
                        continue;
                    }
                    if self.fmark[vi] == id {
                        // FH.10–FH.13 contact: merge K into tree(v)
                        self.relax(&c, &b);
                        let ti = self.ftree[vi] as usize;
                        for &y in &kk {
                            if y != x {
                                trees[ti].edges.push((self.kpar[y as usize], y));
                            }
                            self.fmark[y as usize] = id;
                            self.ftree[y as usize] = ti as u32;
                        }
                        trees[ti].verts.extend_from_slice(&kk);
                        trees[ti].edges.push((u, v));
                        contact = true;
                        break 'search;
                    }
                    let ok = self.relax(&c, &b);
                    if self.kst[vi] != sid {
                        self.kst[vi] = sid;
                        kk.push(v);
                        self.kpar[vi] = u;
                        if ok {
                            if self.valst[vi] != sid {
                                self.valst[vi] = sid;
                                val.push(v);
                            }
                            h.push(v);
                            self.hst[vi] = sid;
                        }
                        if kk.len() >= k {
                            break;
                        }
                    } else if ok {
                        if self.valst[vi] != sid {
                            self.valst[vi] = sid;
                            val.push(v);
                        }
                        if self.hst[vi] != sid {
                            h.push(v);
                            self.hst[vi] = sid;
                        }
                    }
                    prev = e;
                    e = next;
                }
            }
            for &y in &h {
                self.hst[y as usize] = 0;
            }
            if contact {
                continue;
            }
            if kk.len() >= k {
                let ti = trees.len() as u32;
                let mut edges = Vec::with_capacity(kk.len());
                for &y in &kk {
                    if y != x {
                        edges.push((self.kpar[y as usize], y));
                    }
                    self.fmark[y as usize] = id;
                    self.ftree[y as usize] = ti;
                }
                trees.push(Tree { verts: kk.clone(), edges });
            } else {
                for &v in &val {
                    if self.wst[v as usize] != id {
                        self.wst[v as usize] = id;
                        w_out.push(v);
                    }
                }
                q.push(x);
            }
        }
        // MakePivots MP.1–MP.10
        let sid = self.ids.next();
        for &x in s {
            self.ins[x as usize] = sid;
        }
        for &x in &q {
            self.inq[x as usize] = sid;
        }
        let aid = self.ids.next();
        let mut pl: Vec<Vec<u32>> = Vec::new();
        for t in &trees {
            for f in partition(t, k, &mut self.loc, &mut self.ids) {
                let mut cur = Vec::new();
                for x in f {
                    let xi = x as usize;
                    if self.ins[xi] == sid && self.inq[xi] != sid && self.asg[xi] != aid {
                        self.asg[xi] = aid;
                        cur.push(x);
                    }
                }
                if !cur.is_empty() {
                    pl.push(cur);
                }
            }
        }
        (pl, w_out)
    }

    fn argmin_group(&self, g: &[u32], call: u32, j: u32) -> Option<u32> {
        let mut best: Option<u32> = None;
        for &x in g {
            if self.pm_get(x, call) == Some(j) {
                best = match best {
                    None => Some(x),
                    Some(bx) => Some(if self.d[x as usize].lt(&self.d[bx as usize]) { x } else { bx }),
                };
            }
        }
        best
    }

    /// BM.1–BM.31 (PCv2.3 with C-HD's M1, M3, M4, M6).
    fn bmssp(&mut self, b: Label, s: Vec<u32>, l: u32) -> (Label, Vec<u32>, BlockDs) {
        self.stats.calls += 1;
        let tau = self.lambda(l);
        if l == 0 {
            return self.base_case(b, s, tau);
        }
        let call = self.ids.next();
        let mut dd = BlockDs::new(self.mpull(l), b);
        let (pl, w) = self.find_pivots(b, &s);
        let p = pl.len();
        let mut piv = vec![NONE; p];
        let mut cnt = vec![0usize; p];
        let mut bp = b;
        for j in 0..p {
            for &x in &pl[j] {
                self.pm_push(x, call, j as u32);
            }
            cnt[j] = pl[j].len();
            let x = self.argmin_group(&pl[j], call, j as u32).unwrap();
            piv[j] = x;
            let dx = self.d[x as usize];
            dd.insert(&mut self.ds, x, dx);
            bp = bp.min(dx);
        }
        let mut u_all: Vec<u32> = Vec::new();
        let mut jset: Vec<usize> = Vec::new();
        while (u_all.len() as u64) <= tau && !dd.is_empty(&self.ds) {
            let (keys, bi) = dd.pull(&mut self.ds);
            let ssid = self.ids.next();
            let mut si = Vec::with_capacity(keys.len());
            for &x in &keys {
                self.sist[x as usize] = ssid;
                si.push(x);
            }
            for &x in &keys {
                if let Some(j) = self.pm_get(x, call) {
                    if piv[j as usize] == x {
                        for &v in &pl[j as usize] {
                            let vi = v as usize;
                            if self.sist[vi] != ssid && self.pm_get(v, call) == Some(j) && self.d[vi].lt(&bi) {
                                self.sist[vi] = ssid;
                                si.push(v);
                            }
                        }
                    }
                }
            }
            let (bpi, ui, di) = self.bmssp(bi, si, l - 1);
            dd.merge(di);
            for &u in &ui {
                self.ds.delete(u);
            }
            for &u in &ui {
                if let Some(j) = self.pm_get(u, call) {
                    self.pm_pop(u, call);
                    let j = j as usize;
                    cnt[j] -= 1;
                    if piv[j] == u && cnt[j] > 0 {
                        jset.push(j);
                    }
                }
            }
            // BM.19–BM.21 with M1 pointer scans
            for &u in &ui {
                let end = self.g.st[u as usize + 1] as usize;
                let mut e = self.ptr[u as usize] as usize;
                while e < end {
                    let c = self.cand(u, e);
                    if !c.lt(&b) {
                        break;
                    }
                    if self.relax(&c, &b) && !c.lt(&bi) {
                        let v = c.v;
                        let dv = self.d[v as usize];
                        dd.insert(&mut self.ds, v, dv);
                    }
                    e += 1;
                }
                self.ptr[u as usize] = e as u32;
            }
            // BM.23 re-selection
            for &j in &jset {
                if cnt[j] > 0 {
                    let x = self.argmin_group(&pl[j], call, j as u32).unwrap();
                    piv[j] = x;
                    let dx = self.d[x as usize];
                    dd.insert(&mut self.ds, x, dx);
                }
            }
            jset.clear();
            bp = bpi;
            for &u in &ui {
                self.umark[u as usize] = call;
            }
            u_all.extend_from_slice(&ui);
        }
        if dd.is_empty(&self.ds) {
            bp = b; // BM.24a
        }
        // BM.25
        for &x in &s {
            let dx = self.d[x as usize];
            if bp.le(&dx) && dx.lt(&b) {
                dd.insert(&mut self.ds, x, dx);
            }
        }
        // BM.26–BM.29
        let wp: Vec<u32> = w
            .into_iter()
            .filter(|&x| self.umark[x as usize] != call && self.d[x as usize].lt(&bp))
            .collect();
        for &u in &wp {
            let end = self.g.st[u as usize + 1] as usize;
            let mut e = self.g.st[u as usize] as usize;
            while e < end {
                let c = self.cand(u, e);
                if !c.lt(&b) {
                    break;
                }
                if self.relax(&c, &b) && !c.lt(&bp) {
                    let v = c.v;
                    let dv = self.d[v as usize];
                    dd.insert(&mut self.ds, v, dv);
                }
                e += 1;
            }
            self.ptr[u as usize] = e as u32;
        }
        for &u in &wp {
            self.ds.delete(u);
        }
        // BM.30
        for j in 0..p {
            for &x in &pl[j] {
                self.pm_pop(x, call);
            }
        }
        u_all.extend_from_slice(&wp);
        (bp, u_all, dd)
    }

    /// BC.1–BC.9: bounded Dijkstra on a binary heap, leftover keys converted to a block DS.
    fn base_case(&mut self, b: Label, s: Vec<u32>, tau: u64) -> (Label, Vec<u32>, BlockDs) {
        self.stats.base_calls += 1;
        let bid = self.ids.next();
        let mut heap: BinaryHeap<HeapItem> = BinaryHeap::with_capacity(s.len() * 2);
        for &x in &s {
            heap.push(HeapItem(self.d[x as usize], x));
        }
        let mut u_out = Vec::new();
        loop {
            while let Some(top) = heap.peek() {
                let v = top.1 as usize;
                if top.0 != self.d[v] || self.umark[v] == bid {
                    heap.pop();
                } else {
                    break;
                }
            }
            if heap.is_empty() || (u_out.len() as u64) >= tau {
                break;
            }
            let HeapItem(_, u) = heap.pop().unwrap();
            self.umark[u as usize] = bid;
            u_out.push(u);
            let end = self.g.st[u as usize + 1] as usize;
            let mut e = self.g.st[u as usize] as usize;
            while e < end {
                let c = self.cand(u, e);
                if !c.lt(&b) {
                    break;
                }
                if self.relax(&c, &b) {
                    heap.push(HeapItem(self.d[c.v as usize], c.v));
                }
                e += 1;
            }
            self.ptr[u as usize] = e as u32;
        }
        let bp = match heap.peek() {
            None => b,
            Some(top) => top.0,
        };
        let mut dd = BlockDs::new(1, b);
        for HeapItem(lab, v) in heap.into_vec() {
            if lab == self.d[v as usize] && self.umark[v as usize] != bid {
                dd.insert(&mut self.ds, v, lab);
            }
        }
        (bp, u_out, dd)
    }
}

/// PT.1–PT.9: split a tree into connected vertex groups of size in [s, 3s).
fn partition(t: &Tree, s: usize, loc: &mut [u32], ids: &mut Ids) -> Vec<Vec<u32>> {
    let nt = t.verts.len();
    let _ = ids.next();
    for (i, &v) in t.verts.iter().enumerate() {
        loc[v as usize] = i as u32;
    }
    // undirected CSR
    let mut deg = vec![0u32; nt + 1];
    for &(a, b) in &t.edges {
        deg[loc[a as usize] as usize] += 1;
        deg[loc[b as usize] as usize] += 1;
    }
    let mut st = vec![0u32; nt + 1];
    for i in 0..nt {
        st[i + 1] = st[i] + deg[i];
    }
    let mut fill = st.clone();
    let mut adj = vec![0u32; 2 * t.edges.len()];
    for &(a, b) in &t.edges {
        let (la, lb) = (loc[a as usize], loc[b as usize]);
        adj[fill[la as usize] as usize] = lb;
        fill[la as usize] += 1;
        adj[fill[lb as usize] as usize] = la;
        fill[lb as usize] += 1;
    }
    // preorder via iterative DFS
    let mut parent = vec![NONE; nt];
    let mut order = Vec::with_capacity(nt);
    let mut seen = vec![false; nt];
    let mut stack = vec![0u32];
    seen[0] = true;
    while let Some(v) = stack.pop() {
        order.push(v);
        for i in st[v as usize]..st[v as usize + 1] {
            let c = adj[i as usize];
            if !seen[c as usize] {
                seen[c as usize] = true;
                parent[c as usize] = v;
                stack.push(c);
            }
        }
    }
    // post-order accumulation (children before parents)
    let mut acc: Vec<Vec<u32>> = (0..nt).map(|i| vec![i as u32]).collect();
    let mut groups: Vec<Vec<u32>> = Vec::new();
    for &v in order.iter().rev() {
        let vi = v as usize;
        for i in st[vi]..st[vi + 1] {
            let c = adj[i as usize] as usize;
            if parent[c] == v {
                let uc = std::mem::take(&mut acc[c]);
                acc[vi].extend(uc);
                if acc[vi].len() >= s {
                    groups.push(std::mem::replace(&mut acc[vi], vec![v]));
                }
            }
        }
    }
    let rest = std::mem::take(&mut acc[0]);
    if groups.is_empty() {
        groups.push(rest);
    } else {
        groups.last_mut().unwrap().extend(rest);
    }
    groups
        .into_iter()
        .map(|g| g.into_iter().map(|i| t.verts[i as usize]).collect())
        .collect()
}

pub fn chd_sssp(g: &Graph, s: usize, params: ChdParams) -> (Vec<f64>, Stats) {
    let t0 = Instant::now();
    let r = preprocess(g, s);
    let pre = t0.elapsed().as_secs_f64();
    let t1 = Instant::now();
    let nv = r.n;
    let mv = r.head.len();
    // §6 parameters on (cn, cm) = (min(n, m+1), m)
    let cm = g.m();
    let cn = g.n.min(cm + 1).max(1);
    let lgn = (usize::BITS - 1 - nv.max(2).leading_zeros()) as f64;
    let dens = cm.div_ceil(cn).max(1) as f64;
    let tpar = (lgn / dens).powf(2.0 / 3.0).ceil() as u32;
    let t = params.t0.max(tpar).max(1);
    let k = (t as f64).sqrt().ceil() as usize;
    let levels = (lgn as u32) / t + 1;
    let mut hd = vec![NONE; nv];
    let mut nxt = vec![NONE; mv];
    for x in 0..nv {
        let (a, b) = (r.st[x] as usize, r.st[x + 1] as usize);
        if a < b {
            hd[x] = a as u32;
            for e in a..b - 1 {
                nxt[e] = e as u32 + 1;
            }
        }
    }
    let mut c = Chd {
        g: &r,
        d: vec![Label::INF; nv],
        ver: vec![0; nv],
        ptr: r.st[..nv].to_vec(),
        hd,
        nxt,
        ds: DsState::new(nv),
        t,
        k: k.max(1),
        ids: Ids(0),
        pm_head: vec![NONE; nv],
        pm: Vec::new(),
        pm_free: Vec::new(),
        fmark: vec![0; nv],
        ftree: vec![0; nv],
        kst: vec![0; nv],
        kpar: vec![NONE; nv],
        valst: vec![0; nv],
        hst: vec![0; nv],
        wst: vec![0; nv],
        ins: vec![0; nv],
        inq: vec![0; nv],
        asg: vec![0; nv],
        loc: vec![0; nv],
        umark: vec![0; nv],
        sist: vec![0; nv],
        stats: Stats::default(),
    };
    c.d[r.src as usize] = Label::source(r.src);
    let _ = c.bmssp(Label::INF, vec![r.src], levels);
    let mut dist = vec![f64::INFINITY; g.n];
    for v in 0..g.n {
        let o = r.off[v];
        if o != NONE {
            dist[v] = c.d[o as usize].len;
        }
    }
    let mut stats = c.stats;
    stats.n_reduced = nv;
    stats.m_reduced = mv;
    stats.t = t;
    stats.k = k as u32;
    stats.levels = levels;
    stats.preprocess_secs = pre;
    stats.core_secs = t1.elapsed().as_secs_f64();
    (dist, stats)
}
