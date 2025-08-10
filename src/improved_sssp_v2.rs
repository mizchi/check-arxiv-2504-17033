use crate::graph::Graph;
use std::collections::{BinaryHeap, VecDeque};
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

pub fn improved_sssp_v2(graph: &Graph, source: usize) -> Vec<f64> {
    let n = graph.n;
    let mut dist = vec![f64::INFINITY; n];
    dist[source] = 0.0;
    
    let threshold = ((n as f64).powf(2.0 / 3.0)).ceil() as usize;
    
    let mut frontier = BinaryHeap::new();
    frontier.push(Node { id: source, dist: 0.0 });
    
    let mut processed = vec![false; n];
    
    while !frontier.is_empty() {
        if frontier.len() <= threshold {
            dijkstra_phase(&graph, &mut dist, &mut frontier, &mut processed);
        } else {
            bellman_ford_phase(&graph, &mut dist, &mut frontier, &mut processed, threshold);
        }
    }
    
    dist
}

fn dijkstra_phase(
    graph: &Graph,
    dist: &mut Vec<f64>,
    frontier: &mut BinaryHeap<Node>,
    processed: &mut Vec<bool>,
) {
    while let Some(Node { id: u, dist: d }) = frontier.pop() {
        if processed[u] {
            continue;
        }
        processed[u] = true;
        
        if d > dist[u] {
            continue;
        }
        
        for edge in &graph.edges[u] {
            let new_dist = dist[u] + edge.weight;
            if new_dist < dist[edge.to] {
                dist[edge.to] = new_dist;
                frontier.push(Node { id: edge.to, dist: new_dist });
            }
        }
    }
}

fn bellman_ford_phase(
    graph: &Graph,
    dist: &mut Vec<f64>,
    frontier: &mut BinaryHeap<Node>,
    processed: &mut Vec<bool>,
    threshold: usize,
) {
    let mut pivots = Vec::new();
    let mut temp_frontier = BinaryHeap::new();
    
    let k = ((frontier.len() as f64).log2()).ceil() as usize;
    
    for _ in 0..k.min(frontier.len()) {
        if let Some(node) = frontier.pop() {
            if !processed[node.id] {
                pivots.push(node.id);
                temp_frontier.push(node);
            }
        }
    }
    
    let mut queue = VecDeque::new();
    for &pivot in &pivots {
        queue.push_back(pivot);
    }
    
    let mut steps = 0;
    let max_steps = k;
    
    while !queue.is_empty() && steps < max_steps {
        let size = queue.len();
        for _ in 0..size {
            if let Some(u) = queue.pop_front() {
                if processed[u] {
                    continue;
                }
                
                for edge in &graph.edges[u] {
                    let new_dist = dist[u] + edge.weight;
                    if new_dist < dist[edge.to] {
                        dist[edge.to] = new_dist;
                        queue.push_back(edge.to);
                        if !processed[edge.to] {
                            temp_frontier.push(Node { id: edge.to, dist: new_dist });
                        }
                    }
                }
            }
        }
        steps += 1;
    }
    
    for pivot in pivots {
        processed[pivot] = true;
    }
    
    while let Some(node) = temp_frontier.pop() {
        if frontier.len() < threshold * 2 {
            frontier.push(node);
        }
    }
    
    while let Some(node) = frontier.pop() {
        if temp_frontier.len() < threshold * 2 {
            temp_frontier.push(node);
        } else {
            break;
        }
    }
    
    *frontier = temp_frontier;
}