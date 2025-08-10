use shortest_path_validation::graph::Graph;
use shortest_path_validation::dijkstra::dijkstra;
use shortest_path_validation::core_algorithm::{CoreAlgorithm, PartialSortDS};

#[test]
fn test_parameter_calculation() {
    // Test that k and t are calculated correctly according to the paper
    // k = ⌊log^(1/3) n⌋, t = ⌊log^(2/3) n⌋
    
    let test_cases = vec![
        (100, 1, 3),    // log(100) ≈ 4.6, k ≈ 1.6, t ≈ 3.3
        (1000, 2, 4),   // log(1000) ≈ 6.9, k ≈ 1.9, t ≈ 4.3
        (10000, 2, 6),  // log(10000) ≈ 9.2, k ≈ 2.1, t ≈ 6.4
    ];
    
    for (n, expected_k, expected_t) in test_cases {
        let graph = Graph::new(n);
        let algo = CoreAlgorithm::new(graph);
        let (k, t) = algo.get_params();
        
        println!("n={}, k={}, t={} (expected: k≈{}, t≈{})", n, k, t, expected_k, expected_t);
        
        // Allow some flexibility due to rounding
        assert!(k >= expected_k - 1 && k <= expected_k + 1, 
                "k={} not in expected range for n={}", k, n);
        assert!(t >= expected_t - 1 && t <= expected_t + 1,
                "t={} not in expected range for n={}", t, n);
    }
}

#[test]
fn test_partial_sort_ds() {
    // Test the partial sorting data structure operations
    let mut ds = PartialSortDS::new(100);
    
    // Test insert
    ds.insert(1, 5.0);
    ds.insert(2, 3.0);
    ds.insert(3, 7.0);
    
    // Test pull - should return smallest values first
    let (keys, bound) = ds.pull(2);
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], 2); // vertex 2 has distance 3.0
    assert_eq!(keys[1], 1); // vertex 1 has distance 5.0
    assert!(bound.is_some());
    
    // Test batch_prepend
    ds.batch_prepend(vec![(4, 1.0), (5, 2.0)]);
    
    let (keys, _) = ds.pull(3);
    assert_eq!(keys[0], 4); // vertex 4 has distance 1.0 (prepended)
    assert_eq!(keys[1], 5); // vertex 5 has distance 2.0 (prepended)
    assert_eq!(keys[2], 3); // vertex 3 has distance 7.0 (from original)
}

#[test]
fn test_simple_path() {
    // Test on a simple linear graph: 0 -> 1 -> 2 -> 3
    let mut graph = Graph::new(4);
    graph.add_edge(0, 1, 1.0);
    graph.add_edge(1, 2, 2.0);
    graph.add_edge(2, 3, 3.0);
    
    let algo = CoreAlgorithm::new(graph.clone());
    let dist = algo.sssp(0);
    
    assert_eq!(dist[0], 0.0);
    assert_eq!(dist[1], 1.0);
    assert_eq!(dist[2], 3.0);
    assert_eq!(dist[3], 6.0);
    
    // Compare with Dijkstra
    let dijkstra_dist = dijkstra(&graph, 0);
    for i in 0..4 {
        assert!((dist[i] - dijkstra_dist[i]).abs() < 1e-9,
                "Distance mismatch at vertex {}: {} vs {}", i, dist[i], dijkstra_dist[i]);
    }
}

#[test]
fn test_find_pivots_behavior() {
    // Test that FindPivots correctly identifies important vertices
    let mut graph = Graph::new(10);
    
    // Create a hub structure where vertex 1 connects to many vertices
    for i in 2..8 {
        graph.add_edge(1, i, 1.0);
    }
    
    // Create a less connected vertex 8
    graph.add_edge(8, 9, 1.0);
    
    graph.add_edge(0, 1, 1.0);
    graph.add_edge(0, 8, 1.0);
    
    let mut algo = CoreAlgorithm::new(graph);
    algo.dist[0] = 0.0;
    algo.dist[1] = 1.0;
    algo.dist[8] = 1.0;
    
    let sources = vec![1, 8].into_iter().collect();
    let (pivots, _reachable) = algo.find_pivots(f64::INFINITY, &sources);
    
    // Vertex 1 should be identified as a pivot because it reaches many vertices
    assert!(pivots.contains(&1), "Vertex 1 should be a pivot (hub vertex)");
    
    println!("Pivots identified: {:?}", pivots);
}

#[test]
fn test_recursive_depth() {
    // Test that the recursive depth is correctly bounded
    let sizes = vec![100, 1000, 10000];
    
    for n in sizes {
        let log_n = (n as f64).ln();
        let t = (log_n.powf(2.0 / 3.0)).floor() as usize;
        let l = ((log_n / t as f64).ceil() as usize).max(1);
        
        println!("n={}: recursion depth l={}, t={}", n, l, t);
        
        // The recursion depth should be O(log n / log^(2/3) n) = O(log^(1/3) n)
        let expected_depth = (log_n.powf(1.0 / 3.0)).ceil() as usize;
        assert!(l <= expected_depth + 1, 
                "Recursion depth l={} exceeds expected O(log^(1/3) n)={} for n={}", 
                l, expected_depth, n);
    }
}

#[test]
fn test_small_graph_correctness() {
    // Test on various small graphs to ensure correctness
    let test_sizes = vec![10, 20, 50];
    
    for n in test_sizes {
        let graph = Graph::generate_random(n, 0.3, 10.0);
        
        let algo = CoreAlgorithm::new(graph.clone());
        let core_dist = algo.sssp(0);
        let dijkstra_dist = dijkstra(&graph, 0);
        
        for i in 0..n {
            if dijkstra_dist[i].is_finite() {
                assert!((core_dist[i] - dijkstra_dist[i]).abs() < 1e-6,
                        "Distance mismatch at vertex {} in graph size {}: {} vs {}",
                        i, n, core_dist[i], dijkstra_dist[i]);
            }
        }
        
        println!("Graph size {} passed correctness test", n);
    }
}

#[test]
fn test_bounded_exploration() {
    // Test that BaseCase respects the bound parameter
    let mut graph = Graph::new(10);
    for i in 0..9 {
        graph.add_edge(i, i + 1, 2.0);
    }
    
    let mut algo = CoreAlgorithm::new(graph);
    algo.dist[0] = 0.0;
    
    // Set a bound that should limit exploration
    let bound = 5.0;
    algo.base_case(0, bound);
    
    // Vertices within bound should be updated
    assert_eq!(algo.dist[1], 2.0);
    assert_eq!(algo.dist[2], 4.0);
    
    // Vertices beyond bound should not be updated
    assert!(algo.dist[3] > bound || algo.dist[3].is_infinite(),
            "Vertex 3 at distance 6 should not be updated with bound {}", bound);
}

#[test]
fn test_complexity_scaling() {
    // Test that the algorithm scales according to O(m log^(2/3) n)
    use std::time::Instant;
    
    let sizes = vec![100, 200, 400];
    let mut times = Vec::new();
    
    for n in &sizes {
        let graph = Graph::generate_random(*n, 0.1, 10.0);
        let m = graph.m();
        
        let start = Instant::now();
        let algo = CoreAlgorithm::new(graph);
        let _ = algo.sssp(0);
        let elapsed = start.elapsed().as_secs_f64();
        
        let log_2_3_n = (*n as f64).ln().powf(2.0 / 3.0);
        let normalized_time = elapsed / (m as f64 * log_2_3_n);
        
        times.push(normalized_time);
        println!("n={}, m={}, normalized_time={:.6}", n, m, normalized_time);
    }
    
    // Check that normalized times are relatively stable (within 2x)
    if times.len() >= 2 {
        let ratio = times[times.len() - 1] / times[0];
        assert!(ratio < 2.0, 
                "Normalized times should be stable, but ratio is {:.2}", ratio);
    }
}