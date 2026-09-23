
#[derive(Debug, Clone)]
pub struct Edge {
    pub to: usize,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub n: usize,
    pub edges: Vec<Vec<Edge>>,
}

impl Graph {
    pub fn new(n: usize) -> Self {
        Graph {
            n,
            edges: vec![vec![]; n],
        }
    }

    pub fn add_edge(&mut self, from: usize, to: usize, weight: f64) {
        self.edges[from].push(Edge { to, weight });
    }

    pub fn m(&self) -> usize {
        self.edges.iter().map(|e| e.len()).sum()
    }

    pub fn generate_random(n: usize, density: f64, max_weight: f64) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut graph = Graph::new(n);
        
        for i in 0..n {
            for j in 0..n {
                if i != j && rng.gen::<f64>() < density {
                    let weight = rng.gen::<f64>() * max_weight;
                    graph.add_edge(i, j, weight);
                }
            }
        }
        
        graph
    }
}
impl Graph {
    /// Random directed graph with exactly `deg` out-edges per vertex (heads uniform, no
    /// self-loops) and integer weights in [1, max_weight]. O(n·deg), deterministic by `seed`.
    pub fn generate_sparse(n: usize, deg: usize, max_weight: u64, seed: u64) -> Self {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut graph = Graph::new(n);
        for u in 0..n {
            graph.edges[u].reserve(deg);
            for _ in 0..deg {
                let mut v = rng.gen_range(0..n);
                while v == u && n > 1 {
                    v = rng.gen_range(0..n);
                }
                let w = rng.gen_range(1..=max_weight) as f64;
                graph.add_edge(u, v, w);
            }
        }
        graph
    }

    /// side×side directed grid (4-neighbour, both directions) with integer weights in [1, max_weight].
    pub fn generate_grid(side: usize, max_weight: u64, seed: u64) -> Self {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let n = side * side;
        let mut graph = Graph::new(n);
        for r in 0..side {
            for c in 0..side {
                let u = r * side + c;
                let mut add = |v: usize, g: &mut Graph| {
                    let w = rng.gen_range(1..=max_weight) as f64;
                    g.add_edge(u, v, w);
                };
                if c + 1 < side { add(u + 1, &mut graph); }
                if c > 0 { add(u - 1, &mut graph); }
                if r + 1 < side { add(u + side, &mut graph); }
                if r > 0 { add(u - side, &mut graph); }
            }
        }
        graph
    }
}

/// Compressed sparse row copy of a `Graph` (for the tuned Dijkstra baseline).
pub struct Csr {
    pub n: usize,
    pub st: Vec<u32>,
    pub head: Vec<u32>,
    pub w: Vec<f64>,
}

impl Csr {
    pub fn from_graph(g: &Graph) -> Self {
        let mut st = Vec::with_capacity(g.n + 1);
        let mut head = Vec::with_capacity(g.m());
        let mut w = Vec::with_capacity(g.m());
        st.push(0);
        for u in 0..g.n {
            for e in &g.edges[u] {
                head.push(e.to as u32);
                w.push(e.weight);
            }
            st.push(head.len() as u32);
        }
        Csr { n: g.n, st, head, w }
    }
}
