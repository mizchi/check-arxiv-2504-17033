//! Re-check the repository's original benchmark graphs (dense G(n,p)) with an exact comparison
//! that also counts vertices a method leaves at +inf (src/main.rs only compared finite pairs).
use shortest_path_validation::{graph::Graph, dijkstra::dijkstra, improved_sssp_v2::improved_sssp_v2,
    core_algorithm::CoreAlgorithm, improved_sssp::improved_sssp};
fn main() {
    for n in [100usize, 500, 1000, 2000, 5000] {
        for p in [0.01, 0.05, 0.1] {
            let g = Graph::generate_random(n, p, 100.0);
            let want = dijkstra(&g, 0);
            let cnt = |got: &Vec<f64>| (0..n).filter(|&i| (want[i] - got[i]).abs() > 1e-9 || want[i].is_finite() != got[i].is_finite()).count();
            let v1 = cnt(&improved_sssp(&g, 0));
            let v2 = cnt(&improved_sssp_v2(&g, 0));
            let core = cnt(&CoreAlgorithm::new(g.clone()).sssp(0));
            println!("n={n:5} p={p:.2} wrong vertices: v1={v1} v2={v2} core={core}");
        }
    }
}
