//! Head-to-head benchmark: repository implementations vs. faithful DMM+25 (arXiv 2504.17033)
//! vs. C-HD (vals.ai blog / spicylemonade/c-hd-proof), with bit-exact correctness checks.
//!
//! usage: compare [--lg 14,16,18,20] [--deg auto|2,4,8] [--reps 3] [--repo-max-lg 18]
//!                [--t0 16] [--grid] [--csv results/compare.csv]
use shortest_path_validation::chd::{chd_sssp, formal_branch, ChdParams};
use shortest_path_validation::core_algorithm::CoreAlgorithm;
use shortest_path_validation::dijkstra::{dijkstra, dijkstra_csr, dijkstra_dary};
use shortest_path_validation::dmm25::dmm25_sssp;
use shortest_path_validation::graph::{Csr, Graph};
use shortest_path_validation::improved_sssp::improved_sssp;
use shortest_path_validation::improved_sssp_v2::improved_sssp_v2;
use std::io::Write;
use std::time::Instant;

fn arg(args: &[String], k: &str) -> Option<String> {
    args.iter().position(|a| a == k).and_then(|i| args.get(i + 1).cloned())
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn check(want: &[f64], got: &[f64]) -> String {
    let bad = (0..want.len()).filter(|&i| want[i] != got[i]).count();
    if bad == 0 {
        "ok".into()
    } else {
        format!("WRONG({bad})")
    }
}

struct Row {
    graph: String,
    n: usize,
    m: usize,
    algo: String,
    ms: f64,
    status: String,
    note: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let lgs: Vec<u32> = arg(&args, "--lg").unwrap_or("14,16,18,20".into()).split(',').map(|s| s.parse().unwrap()).collect();
    let deg_arg = arg(&args, "--deg").unwrap_or("auto".into());
    let reps: usize = arg(&args, "--reps").unwrap_or("3".into()).parse().unwrap();
    let repo_max: u32 = arg(&args, "--repo-max-lg").unwrap_or("18".into()).parse().unwrap();
    let t0s: Vec<u32> = arg(&args, "--t0").unwrap_or("16".into()).split(',').map(|s| s.parse().unwrap()).collect();
    let csv = arg(&args, "--csv").unwrap_or("results/compare.csv".into());
    let grid = args.iter().any(|a| a == "--grid");
    let skip_dmm = args.iter().any(|a| a == "--skip-dmm25");

    let mut graphs: Vec<(String, Graph, u32)> = Vec::new();
    for &lg in &lgs {
        let n = 1usize << lg;
        if grid {
            let side = (n as f64).sqrt() as usize;
            graphs.push((format!("grid {side}x{side}"), Graph::generate_grid(side, 1 << 20, lg as u64), lg));
            continue;
        }
        let degs: Vec<usize> = if deg_arg == "auto" {
            // 2, the C-HD window [√lg n, ⌊lg^{3/4} n⌋], and 16
            let lo = (lg as f64).sqrt().ceil() as usize;
            let hi = (lg as f64).powf(0.75).floor() as usize;
            let mut v = vec![2, lo, hi, 16];
            v.dedup();
            v
        } else {
            deg_arg.split(',').map(|s| s.parse().unwrap()).collect()
        };
        for d in degs {
            graphs.push((format!("random d={d}"), Graph::generate_sparse(n, d, 1 << 20, (lg as u64) * 1000 + d as u64), lg));
        }
    }

    let mut rows: Vec<Row> = Vec::new();
    println!(
        "{:<16} {:>8} {:>9} | {:<22} {:>10} {:>7}  {}",
        "graph", "n", "m", "algorithm", "ms", "check", "notes"
    );
    for (name, g, lg) in &graphs {
        let n = g.n;
        let m = g.m();
        let csr = Csr::from_graph(g);
        let want = dijkstra_csr(&csr, 0);
        let mut run = |algo: &str, f: &mut dyn FnMut() -> (Vec<f64>, String)| {
            let mut ts = Vec::new();
            let mut out = (Vec::new(), String::new());
            for _ in 0..reps {
                let t = Instant::now();
                out = f();
                ts.push(t.elapsed().as_secs_f64() * 1e3);
            }
            let ms = median(ts);
            let status = check(&want, &out.0);
            println!("{:<16} {:>8} {:>9} | {:<22} {:>10.2} {:>7}  {}", name, n, m, algo, ms, status, out.1);
            rows.push(Row { graph: name.clone(), n, m, algo: algo.into(), ms, status, note: out.1 });
        };
        run("dijkstra-csr (new)", &mut || (dijkstra_csr(&csr, 0), String::new()));
        run("dijkstra (repo)", &mut || (dijkstra(g, 0), String::new()));
        run("dijkstra-4ary (new)", &mut || (dijkstra_dary(&csr, 0), String::new()));
        if *lg <= repo_max {
            run("improved-v1 (repo)", &mut || (improved_sssp(g, 0), String::new()));
            run("improved-v2 (repo)", &mut || (improved_sssp_v2(g, 0), String::new()));
            run("core (repo)", &mut || (CoreAlgorithm::new(g.clone()).sssp(0), String::new()));
        }
        if !skip_dmm { run("dmm25 (new)", &mut || {
            let (d, s) = dmm25_sssp(g, 0);
            (d, format!("N'={} k={} t={} L={} relax/m={:.2} pre={:.0}ms", s.n_reduced, s.k, s.t, s.levels, s.relax_calls as f64 / m as f64, s.preprocess_secs * 1e3))
        }); }
        for &t0 in &t0s {
            run(&format!("chd t0={t0} (new)"), &mut || {
                let (d, s) = chd_sssp(g, 0, ChdParams { t0 });
                (
                    d,
                    format!(
                        "N'={} k={} t={} L={} relax/m={:.2} del={} pre={:.0}ms formal={}",
                        s.n_reduced,
                        s.k,
                        s.t,
                        s.levels,
                        s.relax_calls as f64 / m as f64,
                        s.edge_deletions,
                        s.preprocess_secs * 1e3,
                        formal_branch(n, m)
                    ),
                )
            });
        }
        println!();
    }
    if let Some(dir) = std::path::Path::new(&csv).parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let mut f = std::fs::File::create(&csv).unwrap();
    writeln!(f, "graph,n,m,algorithm,median_ms,check,notes").unwrap();
    for r in rows {
        writeln!(f, "{},{},{},{},{:.3},{},\"{}\"", r.graph, r.n, r.m, r.algo, r.ms, r.status, r.note).unwrap();
    }
    eprintln!("wrote {csv}");
}
