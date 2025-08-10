use crate::graph::Graph;

#[cfg(test)]
use shortest_path_validation::graph::Graph as TestGraph;
use std::collections::{BinaryHeap, VecDeque, HashSet};
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

/// Core implementation following the paper's algorithm
pub struct CoreAlgorithm {
    pub graph: Graph,
    pub dist: Vec<f64>,
    pub k: usize,  // ⌊log^(1/3) n⌋
    pub t: usize,  // ⌊log^(2/3) n⌋
}

impl CoreAlgorithm {
    pub fn new(graph: Graph) -> Self {
        let n = graph.n;
        let log_n = (n as f64).ln().max(1.0);
        
        // Key parameters from the paper
        let k = (log_n.powf(1.0 / 3.0)).ceil() as usize;
        let t = (log_n.powf(2.0 / 3.0)).ceil() as usize;
        
        CoreAlgorithm {
            graph,
            dist: vec![f64::INFINITY; n],
            k: k.max(1),
            t: t.max(1),
        }
    }
    
    /// Main SSSP algorithm
    pub fn sssp(mut self, source: usize) -> Vec<f64> {
        let n = self.graph.n;
        let log_n = (n as f64).ln();
        let l = ((log_n / self.t as f64).ceil() as usize).max(1);
        
        self.dist[source] = 0.0;
        
        let mut frontier = HashSet::new();
        frontier.insert(source);
        
        self.bmssp(l, f64::INFINITY, frontier);
        self.dist
    }
    
    /// Bounded Multi-Source Shortest Path (recursive)
    fn bmssp(&mut self, level: usize, bound: f64, sources: HashSet<usize>) {
        if sources.is_empty() {
            return;
        }
        
        // Base case
        if level == 0 || sources.len() == 1 {
            for &s in &sources {
                self.base_case(s, bound);
            }
            return;
        }
        
        // Find pivots and reduce the frontier
        let (pivots, reachable) = self.find_pivots(bound, &sources);
        
        if pivots.is_empty() {
            return;
        }
        
        // Recursive calls with reduced parameters
        let new_level = level - 1;
        let new_bound = bound / 2.0;
        
        // Process pivots first
        self.bmssp(new_level, new_bound, pivots.clone());
        
        // Process remaining vertices
        let mut remaining = HashSet::new();
        for v in reachable {
            if !pivots.contains(&v) && self.dist[v] < bound {
                remaining.insert(v);
            }
        }
        
        if !remaining.is_empty() {
            self.bmssp(new_level, bound, remaining);
        }
    }
    
    /// FindPivots: Reduces source set to important vertices
    pub fn find_pivots(&mut self, bound: f64, sources: &HashSet<usize>) -> (HashSet<usize>, Vec<usize>) {
        let mut pivots = HashSet::new();
        let mut reachable = Vec::new();
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        
        // Initialize with source vertices
        for &s in sources {
            if self.dist[s] < bound {
                queue.push_back(s);
                visited.insert(s);
            }
        }
        
        // Perform k rounds of relaxation
        for _round in 0..self.k {
            let round_size = queue.len();
            if round_size == 0 {
                break;
            }
            
            for _ in 0..round_size {
                if let Some(u) = queue.pop_front() {
                    // Count vertices reached from u
                    let mut reached_count = 0;
                    
                    for edge in &self.graph.edges[u] {
                        let new_dist = self.dist[u] + edge.weight;
                        
                        if new_dist < bound && new_dist < self.dist[edge.to] {
                            self.dist[edge.to] = new_dist;
                            reached_count += 1;
                            
                            if !visited.contains(&edge.to) {
                                visited.insert(edge.to);
                                queue.push_back(edge.to);
                                reachable.push(edge.to);
                            }
                        }
                    }
                    
                    // Add to pivots if it reaches enough vertices
                    if sources.contains(&u) && reached_count >= self.k {
                        pivots.insert(u);
                    }
                }
            }
        }
        
        // Ensure we don't have too many pivots
        let max_pivots = (sources.len() / self.k).max(1);
        let pivots: HashSet<usize> = pivots.into_iter().take(max_pivots).collect();
        
        (pivots, reachable)
    }
    
    /// BaseCase: Dijkstra-like exploration for small sets
    pub fn base_case(&mut self, source: usize, bound: f64) {
        let mut heap = BinaryHeap::new();
        let mut processed = HashSet::new();
        
        if self.dist[source] < bound {
            heap.push(Node { id: source, dist: self.dist[source] });
        }
        
        // For base case, explore all reachable vertices within bound
        while let Some(Node { id: u, dist: d }) = heap.pop() {
            if processed.contains(&u) || d >= bound {
                continue;
            }
            processed.insert(u);
            
            for edge in &self.graph.edges[u] {
                let new_dist = self.dist[u] + edge.weight;
                
                if new_dist < bound && new_dist < self.dist[edge.to] {
                    self.dist[edge.to] = new_dist;
                    heap.push(Node { id: edge.to, dist: new_dist });
                }
            }
        }
    }
    
    /// Get algorithm parameters for testing
    pub fn get_params(&self) -> (usize, usize) {
        (self.k, self.t)
    }
}

/// Partial sorting data structure from the paper
pub struct PartialSortDS {
    blocks: Vec<Vec<(usize, f64)>>,
    block_size: usize,
}

impl PartialSortDS {
    pub fn new(n: usize) -> Self {
        let block_size = ((n as f64).powf(1.0 / 3.0)).ceil() as usize;
        PartialSortDS {
            blocks: vec![Vec::new()],
            block_size: block_size.max(1),
        }
    }
    
    pub fn insert(&mut self, key: usize, value: f64) {
        if let Some(last_block) = self.blocks.last_mut() {
            last_block.push((key, value));
            last_block.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
            
            if last_block.len() >= self.block_size * 2 {
                let new_block = last_block.split_off(self.block_size);
                self.blocks.push(new_block);
            }
        }
    }
    
    pub fn batch_prepend(&mut self, items: Vec<(usize, f64)>) {
        let mut new_block = items;
        new_block.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        self.blocks.insert(0, new_block);
    }
    
    pub fn pull(&mut self, count: usize) -> (Vec<usize>, Option<f64>) {
        let mut result = Vec::new();
        let mut upper_bound = None;
        
        for block in &mut self.blocks {
            while !block.is_empty() && result.len() < count {
                if let Some((key, value)) = block.first() {
                    result.push(*key);
                    upper_bound = Some(*value);
                    block.remove(0);
                }
            }
            
            if result.len() >= count {
                break;
            }
        }
        
        // Clean up empty blocks
        self.blocks.retain(|b| !b.is_empty());
        
        (result, upper_bound)
    }
}