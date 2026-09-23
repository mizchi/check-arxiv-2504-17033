use crate::graph::Graph;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
struct Node {
    id: usize,
    dist: f64,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.dist == other.dist
    }
}

impl Eq for Node {}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.dist.partial_cmp(&self.dist)
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

pub fn dijkstra(graph: &Graph, source: usize) -> Vec<f64> {
    let n = graph.n;
    let mut dist = vec![f64::INFINITY; n];
    let mut heap = BinaryHeap::new();
    
    dist[source] = 0.0;
    heap.push(Node { id: source, dist: 0.0 });
    
    while let Some(Node { id: u, dist: d }) = heap.pop() {
        if d > dist[u] {
            continue;
        }
        
        for edge in &graph.edges[u] {
            let new_dist = dist[u] + edge.weight;
            
            if new_dist < dist[edge.to] {
                dist[edge.to] = new_dist;
                heap.push(Node { id: edge.to, dist: new_dist });
            }
        }
    }
    
    dist
}
/// Tuned baseline: binary-heap Dijkstra on a CSR graph with (u64 dist bits, u32 id) keys.
/// Nonnegative f64 compare like their bit patterns, so the heap key is exact.
pub fn dijkstra_csr(g: &crate::graph::Csr, source: usize) -> Vec<f64> {
    use std::cmp::Reverse;
    let mut dist = vec![f64::INFINITY; g.n];
    let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
    dist[source] = 0.0;
    heap.push(Reverse((0f64.to_bits(), source as u32)));
    while let Some(Reverse((db, u))) = heap.pop() {
        let du = f64::from_bits(db);
        let u = u as usize;
        if du > dist[u] {
            continue;
        }
        for e in g.st[u] as usize..g.st[u + 1] as usize {
            let v = g.head[e] as usize;
            let nd = du + g.w[e];
            if nd < dist[v] {
                dist[v] = nd;
                heap.push(Reverse((nd.to_bits(), v as u32)));
            }
        }
    }
    dist
}

/// Stronger baseline: Dijkstra with an indexed 4-ary heap and decrease-key (heap size <= n).
pub fn dijkstra_dary(g: &crate::graph::Csr, source: usize) -> Vec<f64> {
    const NIL: u32 = u32::MAX;
    const DONE: u32 = u32::MAX - 1;
    let n = g.n;
    let mut dist = vec![f64::INFINITY; n];
    let mut pos = vec![NIL; n];
    let mut heap: Vec<u32> = Vec::new();
    fn sift_up(heap: &mut [u32], pos: &mut [u32], dist: &[f64], mut i: usize) {
        let x = heap[i];
        let dx = dist[x as usize];
        while i > 0 {
            let p = (i - 1) / 4;
            let y = heap[p];
            if dist[y as usize] <= dx {
                break;
            }
            heap[i] = y;
            pos[y as usize] = i as u32;
            i = p;
        }
        heap[i] = x;
        pos[x as usize] = i as u32;
    }
    fn sift_down(heap: &mut [u32], pos: &mut [u32], dist: &[f64], mut i: usize) {
        let len = heap.len();
        let x = heap[i];
        let dx = dist[x as usize];
        loop {
            let c0 = 4 * i + 1;
            if c0 >= len {
                break;
            }
            let mut best = c0;
            let mut bd = dist[heap[c0] as usize];
            for c in c0 + 1..(c0 + 4).min(len) {
                let dc = dist[heap[c] as usize];
                if dc < bd {
                    bd = dc;
                    best = c;
                }
            }
            if bd >= dx {
                break;
            }
            heap[i] = heap[best];
            pos[heap[i] as usize] = i as u32;
            i = best;
        }
        heap[i] = x;
        pos[x as usize] = i as u32;
    }
    dist[source] = 0.0;
    heap.push(source as u32);
    pos[source] = 0;
    while !heap.is_empty() {
        let u = heap[0] as usize;
        let last = heap.pop().unwrap();
        pos[u] = DONE;
        if !heap.is_empty() {
            heap[0] = last;
            pos[last as usize] = 0;
            sift_down(&mut heap, &mut pos, &dist, 0);
        }
        let du = dist[u];
        for e in g.st[u] as usize..g.st[u + 1] as usize {
            let v = g.head[e] as usize;
            let nd = du + g.w[e];
            if nd < dist[v] {
                dist[v] = nd;
                if pos[v] == NIL {
                    heap.push(v as u32);
                    let i = heap.len() - 1;
                    sift_up(&mut heap, &mut pos, &dist, i);
                } else {
                    let i = pos[v] as usize;
                    sift_up(&mut heap, &mut pos, &dist, i);
                }
            }
        }
    }
    dist
}
