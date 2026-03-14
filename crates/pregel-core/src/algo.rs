//! Algorithm selection and result-extraction metadata.

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

/// Per-worker query: what to extract from a partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultQuery {
    AllVertexValues,
    VertexSubset(Vec<u64>),
}

/// Coordinator post-function: how to combine worker outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostFunction {
    ConcatAndSort,
    Concat,
    SingleValue,
}

/// Metadata for result extraction per algorithm.
#[derive(Debug, Clone)]
pub struct AlgoMetadata {
    pub query: ResultQuery,
    pub post: PostFunction,
}

impl AlgoMetadata {
    pub fn for_algo(algo: Algorithm) -> Self {
        match algo {
            Algorithm::Pagerank => AlgoMetadata {
                query: ResultQuery::AllVertexValues,
                post: PostFunction::Concat,
            },
            Algorithm::ConnectedComponents => AlgoMetadata {
                query: ResultQuery::AllVertexValues,
                post: PostFunction::ConcatAndSort,
            },
            Algorithm::ShortestPath => AlgoMetadata {
                query: ResultQuery::AllVertexValues,
                post: PostFunction::ConcatAndSort,
            },
        }
    }
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
