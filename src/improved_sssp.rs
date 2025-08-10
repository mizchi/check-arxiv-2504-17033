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

pub fn improved_sssp(graph: &Graph, source: usize) -> Vec<f64> {
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