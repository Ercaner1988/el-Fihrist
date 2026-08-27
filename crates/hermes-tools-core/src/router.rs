use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub target: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Graph {
    pub adjacency: HashMap<String, Vec<Edge>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub found: bool,
    pub total_distance: f64,
    pub path: Vec<String>,
}

#[derive(Copy, Clone, PartialEq)]
struct State {
    cost: f64,
    node: usize,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Graph {
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, from: &str, to: &str, weight: f64) {
        self.adjacency.entry(from.to_string()).or_default().push(Edge {
            target: to.to_string(),
            weight,
        });
    }

    pub fn shortest_path(&self, start: &str, goal: &str) -> RouteResult {
        if start == goal {
            return RouteResult {
                found: true,
                total_distance: 0.0,
                path: vec![start.to_string()],
            };
        }

        let nodes: Vec<String> = self.adjacency.keys().cloned().collect();
        let node_map: HashMap<String, usize> = nodes.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();

        let start_idx = match node_map.get(start) {
            Some(&i) => i,
            None => return RouteResult { found: false, total_distance: 0.0, path: Vec::new() },
        };

        let goal_idx = match node_map.get(goal) {
            Some(&i) => i,
            None => return RouteResult { found: false, total_distance: 0.0, path: Vec::new() },
        };

        let mut dist: Vec<f64> = vec![f64::INFINITY; nodes.len()];
        let mut prev: Vec<Option<usize>> = vec![None; nodes.len()];
        let mut heap = BinaryHeap::new();

        dist[start_idx] = 0.0;
        heap.push(State { cost: 0.0, node: start_idx });

        while let Some(State { cost, node }) = heap.pop() {
            if cost > dist[node] {
                continue;
            }

            if node == goal_idx {
                break;
            }

            let u_name = &nodes[node];
            if let Some(edges) = self.adjacency.get(u_name) {
                for edge in edges {
                    if let Some(&v_idx) = node_map.get(&edge.target) {
                        let next_cost = cost + edge.weight;
                        if next_cost < dist[v_idx] {
                            dist[v_idx] = next_cost;
                            prev[v_idx] = Some(node);
                            heap.push(State { cost: next_cost, node: v_idx });
                        }
                    }
                }
            }
        }

        if dist[goal_idx].is_infinite() {
            return RouteResult {
                found: false,
                total_distance: 0.0,
                path: Vec::new(),
            };
        }

        let mut path = Vec::new();
        let mut curr = Some(goal_idx);
        while let Some(u) = curr {
            path.push(nodes[u].clone());
            curr = prev[u];
        }
        path.reverse();

        RouteResult {
            found: true,
            total_distance: dist[goal_idx],
            path,
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn graf_en_kisa_yol_dijkstra() {
        let mut g = Graph::new();
        g.add_edge("A", "B", 4.0);
        g.add_edge("A", "C", 2.0);
        g.add_edge("C", "B", 1.0);
        g.add_edge("B", "D", 5.0);
        g.add_edge("C", "D", 8.0);

        let route = g.shortest_path("A", "B");
        assert!(route.found);
        assert_eq!(route.total_distance, 3.0); // A -> C -> B (2 + 1 = 3)
        assert_eq!(route.path, vec!["A", "C", "B"]);
    }
}
