//! Comparison counts in the comparison-addition model (run with `--features count-cmp`):
//! binary-heap Dijkstra vs. C-HD vs. DMM+25 on random d-out-regular graphs.
use shortest_path_validation::chd::{chd_sssp, ChdParams};
use shortest_path_validation::common::{count_cmp, CMP_COUNT};
use shortest_path_validation::dmm25::dmm25_sssp;
use shortest_path_validation::graph::{Csr, Graph};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::atomic::Ordering::Relaxed;

#[derive(PartialEq)]
struct Item(f64, u32);
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        count_cmp();
        o.0.partial_cmp(&self.0).unwrap()
    }
}

fn dijkstra_counting(g: &Csr, s: usize) -> Vec<f64> {
    let mut dist = vec![f64::INFINITY; g.n];
    let mut h = BinaryHeap::new();
    dist[s] = 0.0;
    h.push(Item(0.0, s as u32));
    while let Some(Item(du, u)) = h.pop() {
        let u = u as usize;
        count_cmp();
        if du > dist[u] {
            continue;
        }
        for e in g.st[u] as usize..g.st[u + 1] as usize {
            let v = g.head[e] as usize;
            let nd = du + g.w[e];
            count_cmp();
            if nd < dist[v] {
                dist[v] = nd;
                h.push(Item(nd, v as u32));
            }
        }
    }
    dist
}

fn take() -> u64 {
    CMP_COUNT.swap(0, Relaxed)
}

fn main() {
    println!("| n | d | m | Dijkstra cmp/m | C-HD t0=16 cmp/m | C-HD / Dijkstra | DMM+25 cmp/m |");
    println!("|---|---:|---:|---:|---:|---:|---:|");
    for lg in [14u32, 16, 18, 20, 22] {
        for d in [4usize, 8, 16] {
            let n = 1usize << lg;
            let g = Graph::generate_sparse(n, d, 1 << 20, lg as u64 * 1000 + d as u64);
            let m = g.m() as f64;
            let csr = Csr::from_graph(&g);
            take();
            let want = dijkstra_counting(&csr, 0);
            let cd = take() as f64;
            let (got, _) = chd_sssp(&g, 0, ChdParams { t0: 16 });
            let cc = take() as f64;
            assert_eq!(want, got);
            let dm = if lg <= 18 {
                let (got, _) = dmm25_sssp(&g, 0);
                assert_eq!(want, got);
                format!("{:.1}", take() as f64 / m)
            } else {
                "—".into()
            };
            println!("| 2^{lg} | {d} | {} | {:.2} | {:.2} | {:.2}× | {dm} |", g.m(), cd / m, cc / m, cc / cd);
        }
    }
}
