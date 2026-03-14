# pregel-cli

**Purpose:** Command-line interface for the Pregel framework. Submit jobs, build programs, manage local clusters, and initialize new projects.

**Usage:** Run `cargo run -p pregel-cli -- <command>` or install and run `pregel <command>`.

**Note:** Run from the `pregel/` directory (workspace root) so Cargo finds `Cargo.toml`.

## Commands

| Command | Purpose |
|--------|---------|
| `submit` | Submit a job: WASM program + graph + worker count |
| `build` | Build a Pregel program from source (compiles to WASM) |
| `cluster` | Manage a local cluster: start, stop, status |
| `init` | Create a new Pregel project scaffold |

## Examples

```bash
# Submit a PageRank job with 8 workers
pregel submit --program pagerank.wasm --graph s3://graphs/webgraph --workers 8

# Start a local development cluster
pregel cluster start

# Initialize a new project
pregel init connected_components
```

## Rust Note: clap

We use [clap](https://docs.rs/clap) with the derive API. The `#[derive(Parser)]` and `#[derive(Subcommand)]` macros generate argument parsing from the struct definitions. Each `///` doc comment becomes the `--help` text for that option.
