mod graph;
mod core_algorithm;

use graph::Graph;
use core_algorithm::CoreAlgorithm;

fn main() {
    let mut graph = Graph::new(4);
    graph.add_edge(0, 1, 1.0);
    graph.add_edge(1, 2, 2.0);
    graph.add_edge(2, 3, 3.0);
    
    let mut algo = CoreAlgorithm::new(graph);
    println!("Initial dist: {:?}", algo.dist);
    println!("k={}, t={}", algo.k, algo.t);
    
    algo.dist[0] = 0.0;
    
    // Manually run base_case
    algo.base_case(0, f64::INFINITY);
    println!("After base_case(0): {:?}", algo.dist);
    
    // Check the main algorithm
    let mut graph2 = Graph::new(4);
    graph2.add_edge(0, 1, 1.0);
    graph2.add_edge(1, 2, 2.0);
    graph2.add_edge(2, 3, 3.0);
    
    let algo2 = CoreAlgorithm::new(graph2);
    let dist = algo2.sssp(0);
    println!("Final dist from sssp: {:?}", dist);
}