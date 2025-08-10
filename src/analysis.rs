mod graph;
mod dijkstra;
mod improved_sssp;
mod improved_sssp_v2;

use graph::Graph;
use dijkstra::dijkstra;
use improved_sssp::improved_sssp;
use improved_sssp_v2::improved_sssp_v2;
use std::time::Instant;

fn main() {
    println!("Detailed Performance Analysis\n");
    println!("{}", "=".repeat(100));
    
    // Test sparse graphs
    println!("\n1. SPARSE GRAPHS (density = 0.01)");
    println!("{}", "-".repeat(100));
    test_density(0.01);
    
    // Test medium density graphs
    println!("\n2. MEDIUM DENSITY GRAPHS (density = 0.05)");
    println!("{}", "-".repeat(100));
    test_density(0.05);
    
    // Test dense graphs
    println!("\n3. DENSE GRAPHS (density = 0.2)");
    println!("{}", "-".repeat(100));
    test_density(0.2);
    
    // Test complexity scaling
    println!("\n4. COMPLEXITY SCALING ANALYSIS");
    println!("{}", "-".repeat(100));
    complexity_analysis();
}

fn test_density(density: f64) {
    println!("{:<10} {:<10} {:<15} {:<15} {:<15} {:<10}", 
             "Nodes", "Edges", "Dijkstra (ms)", "Improved (ms)", "ImprovedV2 (ms)", "Best Speedup");
    
    let sizes = vec![100, 500, 1000, 2000, 3000, 4000, 5000];
    
    for n in sizes {
        let graph = Graph::generate_random(n, density, 100.0);
        let m = graph.m();
        
        let (d_time, i1_time, i2_time) = benchmark_algorithms(&graph);
        let best_improved = i1_time.min(i2_time);
        let speedup = d_time / best_improved;
        
        println!("{:<10} {:<10} {:<15.3} {:<15.3} {:<15.3} {:<10.2}x", 
                 n, m, d_time, i1_time, i2_time, speedup);
    }
}

fn complexity_analysis() {
    println!("Testing theoretical complexity O(m log^(2/3) n) vs O(m log n)");
    println!();
    println!("{:<10} {:<10} {:<20} {:<20} {:<15}", 
             "Nodes", "Edges", "Dijkstra/m*log(n)", "ImprovedV2/m*log^⅔(n)", "Ratio");
    
    let density = 0.05;
    let sizes = vec![500, 1000, 2000, 4000, 8000];
    
    for n in sizes {
        let graph = Graph::generate_random(n, density, 100.0);
        let m = graph.m();
        
        let (d_time, _, i2_time) = benchmark_algorithms(&graph);
        
        let log_n = (n as f64).ln();
        let log_2_3_n = log_n.powf(2.0 / 3.0);
        
        let dijkstra_normalized = d_time / (m as f64 * log_n);
        let improved_normalized = i2_time / (m as f64 * log_2_3_n);
        let ratio = dijkstra_normalized / improved_normalized;
        
        println!("{:<10} {:<10} {:<20.6} {:<20.6} {:<15.3}", 
                 n, m, dijkstra_normalized, improved_normalized, ratio);
    }
}

fn benchmark_algorithms(graph: &Graph) -> (f64, f64, f64) {
    // Run multiple times for accuracy
    let runs = 3;
    let mut d_times = Vec::new();
    let mut i1_times = Vec::new();
    let mut i2_times = Vec::new();
    
    for _ in 0..runs {
        let start = Instant::now();
        let _ = dijkstra(&graph, 0);
        d_times.push(start.elapsed().as_secs_f64() * 1000.0);
        
        let start = Instant::now();
        let _ = improved_sssp(&graph, 0);
        i1_times.push(start.elapsed().as_secs_f64() * 1000.0);
        
        let start = Instant::now();
        let _ = improved_sssp_v2(&graph, 0);
        i2_times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    
    let d_avg = d_times.iter().sum::<f64>() / runs as f64;
    let i1_avg = i1_times.iter().sum::<f64>() / runs as f64;
    let i2_avg = i2_times.iter().sum::<f64>() / runs as f64;
    
    (d_avg, i1_avg, i2_avg)
}