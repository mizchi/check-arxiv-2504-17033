use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use shortest_path_validation::graph::Graph;
use shortest_path_validation::dijkstra::dijkstra;
use shortest_path_validation::improved_sssp::improved_sssp;

fn benchmark_algorithms(c: &mut Criterion) {
    let sizes = vec![100, 500, 1000, 2000];
    let density = 0.1;
    let max_weight = 100.0;
    
    let mut group = c.benchmark_group("shortest_path");
    
    for n in sizes {
        let graph = Graph::generate_random(n, density, max_weight);
        let m = graph.m();
        
        group.bench_with_input(
            BenchmarkId::new("Dijkstra", format!("n={}, m={}", n, m)),
            &graph,
            |b, g| {
                b.iter(|| dijkstra(black_box(g), black_box(0)));
            }
        );
        
        group.bench_with_input(
            BenchmarkId::new("Improved", format!("n={}, m={}", n, m)),
            &graph,
            |b, g| {
                b.iter(|| improved_sssp(black_box(g), black_box(0)));
            }
        );
    }
    
    group.finish();
}

fn benchmark_sparse_dense(c: &mut Criterion) {
    let n = 1000;
    let densities = vec![0.01, 0.05, 0.1, 0.2];
    let max_weight = 100.0;
    
    let mut group = c.benchmark_group("density_comparison");
    
    for density in densities {
        let graph = Graph::generate_random(n, density, max_weight);
        let m = graph.m();
        
        group.bench_with_input(
            BenchmarkId::new("Dijkstra", format!("density={:.2}, m={}", density, m)),
            &graph,
            |b, g| {
                b.iter(|| dijkstra(black_box(g), black_box(0)));
            }
        );
        
        group.bench_with_input(
            BenchmarkId::new("Improved", format!("density={:.2}, m={}", density, m)),
            &graph,
            |b, g| {
                b.iter(|| improved_sssp(black_box(g), black_box(0)));
            }
        );
    }
    
    group.finish();
}

criterion_group!(benches, benchmark_algorithms, benchmark_sparse_dense);
criterion_main!(benches);