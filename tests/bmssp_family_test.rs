//! Exactness of the faithful BMSSP-family implementations (DMM+25 and C-HD) against Dijkstra,
//! compared bit-for-bit (integer weights make every sum exact), including unreachable vertices.
use rand::{Rng, SeedableRng};
use shortest_path_validation::chd::{chd_sssp, ChdParams};
use shortest_path_validation::dijkstra::dijkstra;
use shortest_path_validation::dmm25::dmm25_sssp;
use shortest_path_validation::graph::Graph;

fn random_graph(rng: &mut rand::rngs::StdRng, n: usize, m: usize, maxw: u64) -> Graph {
    let mut g = Graph::new(n);
    for _ in 0..m {
        let u = rng.gen_range(0..n);
        let v = rng.gen_range(0..n); // self-loops and parallel edges allowed
        g.add_edge(u, v, rng.gen_range(0..=maxw) as f64); // zero weights allowed
    }
    g
}

fn diff(want: &[f64], got: &[f64]) -> Option<String> {
    let bad: Vec<usize> = (0..want.len()).filter(|&i| want[i] != got[i]).collect();
    if bad.is_empty() {
        None
    } else {
        let i = bad[0];
        Some(format!("{} of {} differ; first v={i}: want {} got {}", bad.len(), want.len(), want[i], got[i]))
    }
}

fn check(g: &Graph, s: usize, tag: &str) {
    let want = dijkstra(g, s);
    let (d1, _) = dmm25_sssp(g, s);
    if let Some(msg) = diff(&want, &d1) {
        panic!("dmm25 mismatch ({tag}): {msg}");
    }
    for t0 in [1, 2, 4, 9, 16] {
        let (d2, _) = chd_sssp(g, s, ChdParams { t0 });
        if let Some(msg) = diff(&want, &d2) {
            panic!("chd t0={t0} mismatch ({tag}): {msg}");
        }
    }
}

#[test]
fn small_random_graphs_with_ties() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);
    for it in 0..3000 {
        let n = rng.gen_range(1..40);
        let m = rng.gen_range(0..n * 4 + 1);
        let maxw = [0u64, 1, 3, 100][it % 4];
        let g = random_graph(&mut rng, n, m, maxw);
        let s = rng.gen_range(0..n);
        check(&g, s, &format!("it={it} n={n} m={m} maxw={maxw}"));
    }
}

#[test]
fn medium_random_graphs() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(2);
    for it in 0..60 {
        let n = rng.gen_range(200..3000);
        let deg = rng.gen_range(1..10);
        let maxw = [2u64, 50, 1 << 20][it % 3];
        let g = random_graph(&mut rng, n, n * deg, maxw);
        check(&g, 0, &format!("it={it} n={n} deg={deg}"));
    }
}

#[test]
fn grids_and_regular_graphs() {
    for (i, side) in [3usize, 10, 40, 90].into_iter().enumerate() {
        let g = Graph::generate_grid(side, 1 + i as u64 * 7, i as u64);
        check(&g, 0, &format!("grid {side}"));
    }
    for deg in [1usize, 2, 4, 8] {
        let g = Graph::generate_sparse(20_000, deg, 1 << 20, deg as u64);
        check(&g, 0, &format!("sparse deg={deg}"));
    }
}
