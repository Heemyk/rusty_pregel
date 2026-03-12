//! Algorithm selection.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Algorithm {
    #[default]
    Pagerank,
    ConnectedComponents,
    /// Single-source shortest path (unweighted). Source vertex = 0.
    ShortestPath,
}

impl std::str::FromStr for Algorithm {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pagerank" | "pr" => Ok(Self::Pagerank),
            "connected_components" | "cc" => Ok(Self::ConnectedComponents),
            "shortest_path" | "sssp" | "sp" => Ok(Self::ShortestPath),
            _ => Err(format!("Unknown algorithm: {}", s)),
        }
    }
}
