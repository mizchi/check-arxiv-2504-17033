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
    println!("Shortest Path Algorithm Validation\n");
    println!("Testing on various graph sizes:");
    println!("{:<10} {:<10} {:<15} {:<15} {:<15} {:<10}", "Nodes", "Edges", "Dijkstra (ms)", "Improved (ms)", "ImprovedV2 (ms)", "Best Speedup");
    println!("{}", "-".repeat(80));
    
    let densities = vec![0.05, 0.1];
    let sizes = vec![100, 500, 1000, 2000, 5000];
    
    for n in sizes {
        for &density in &densities {
            let graph = Graph::generate_random(n, density, 100.0);
            let m = graph.m();
            
            let start = Instant::now();
            let dist1 = dijkstra(&graph, 0);
            let dijkstra_time = start.elapsed().as_secs_f64() * 1000.0;
            
            let start = Instant::now();
            let dist2 = improved_sssp(&graph, 0);
            let improved_time = start.elapsed().as_secs_f64() * 1000.0;
            
            let start = Instant::now();
            let dist3 = improved_sssp_v2(&graph, 0);
            let improved_v2_time = start.elapsed().as_secs_f64() * 1000.0;
            
            let best_improved = improved_time.min(improved_v2_time);
            let speedup = dijkstra_time / best_improved;
            
            println!("{:<10} {:<10} {:<15.3} {:<15.3} {:<15.3} {:<10.2}x", 
                     n, m, dijkstra_time, improved_time, improved_v2_time, speedup);
            
            let mut max_diff = 0.0f64;
            for i in 0..n {
                if dist1[i].is_finite() && dist2[i].is_finite() {
                    max_diff = max_diff.max((dist1[i] - dist2[i]).abs());
                }
                if dist1[i].is_finite() && dist3[i].is_finite() {
                    max_diff = max_diff.max((dist1[i] - dist3[i]).abs());
                }
            }
            
            if max_diff > 1e-9 {
                println!("WARNING: Results differ by {:.2e}", max_diff);
            }
        }
    }
    
    println!("\nComplexity Analysis:");
    println!("Dijkstra: O(m log n) = O(m + n log n)");
    println!("Improved V1: Simple implementation");
    println!("Improved V2: Dijkstra-Bellman-Ford hybrid with adaptive frontier management");
    println!("Target: O(m log^(2/3) n) [claimed in paper]");
}
