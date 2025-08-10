
#[derive(Debug, Clone)]
pub struct Edge {
    pub to: usize,
    pub weight: f64,
}

#[derive(Debug)]
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