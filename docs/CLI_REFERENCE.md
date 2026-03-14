# Pregel CLI Reference

Full command and flag reference. Run from the `pregel/` directory.

**Usage:** `cargo run -p pregel-cli -- <command> [OPTIONS]` or `pregel <command> [OPTIONS]` (if installed)

---

## Commands Overview

| Command | Purpose |
|---------|---------|
| `session` | Start coordinator + workers; graph loaded; accepts multiple jobs |
| `job` | Submit a job to an existing session (HTTP) |
| `submit` | Single-shot: start, run one job, exit |
| `build` | Build a Pregel program (stub) |
| `cluster` | Cluster management: start, stop, status |
| `init` | Initialize new project (stub) |

---

## session

Start coordinator and workers. Graph is loaded and partitioned. Workers stay alive between jobs. Submit jobs with `pregel job --session <url>`.

```bash
pregel session --graph <PATH> [OPTIONS]
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--graph` | `-g` | *required* | Path to graph file (edge list: "src dst" per line) |
| `--workers` | `-w` | 2 | Number of workers |
| `--host` | | 127.0.0.1 | Host for binding |
| `--coordinator-port` | | 5000 | Coordinator gRPC port |
| `--http-port` | | 5100 | Coordinator HTTP API port (for job submission) |
| `--transport` | `-t` | tcp | Transport: tcp or quic |
| `--checkpoint-dir` | | | Dir for periodic checkpoints (every 10 supersteps) |
| `--metrics-port` | | | Base port for /metrics (workers get base, base+1, ...) |
| `--worker-timeout` | | 60 | Worker report timeout (sec); coordinator aborts job if exceeded |
| `--verbose` | `-v` | 0 | Verbosity: 0=quiet, 1=summary, 2=full dumps |

**Example:**
```bash
pregel session --graph examples/data/sample.graph --workers 2 --metrics-port 9090
```

---

## job

Submit a job to an existing session. Session must be running (from `pregel session`).

```bash
pregel job --session <URL> [OPTIONS]
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--session` | `-s` | *required* | Session HTTP URL (e.g. http://127.0.0.1:5100) |
| `--algo` | `-a` | cc | Algorithm: cc, pagerank, shortest_path (aliases: pr, sssp, sp) |
| `--program` | `-p` | | Path to WASM module (optional; native algo if omitted) |

**Example:**
```bash
pregel job --session http://127.0.0.1:5100 --algo cc
pregel job --session http://127.0.0.1:5100 --algo pagerank --program pagerank.wasm
```

---

## submit

Single-shot: start coordinator and workers, run one algorithm, then exit. No session persistence.

```bash
pregel submit --graph <PATH> [OPTIONS]
```

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--graph` | `-g` | *required* | Path to graph file |
| `--workers` | `-w` | 2 | Number of workers |
| `--host` | | 127.0.0.1 | Host for binding |
| `--coordinator-port` | | 5000 | Coordinator gRPC port |
| `--algo` | `-a` | pagerank | Algorithm: cc, pagerank, shortest_path |
| `--transport` | `-t` | tcp | Transport: tcp or quic |
| `--program` | `-p` | | Path to WASM module (optional) |
| `--checkpoint-dir` | | | Dir for periodic checkpoints |
| `--metrics-port` | | | Base port for /metrics |
| `--worker-timeout` | | 60 | Worker report timeout (sec) |
| `--verbose` | `-v` | 0 | Verbosity: 0, 1, 2 |

**Example:**
```bash
pregel submit --graph examples/data/sample.graph --algo cc --workers 2
```

---

## build

Build a Pregel program from source. (Stub: run `cargo build` in project dir for WASM.)

```bash
pregel build [PATH]
```

| Arg | Default | Description |
|-----|---------|-------------|
| PATH | . | Project directory |

---

## cluster

Manage local cluster lifecycle.

```bash
pregel cluster [start|stop|status]
```

| Subcommand | Description |
|------------|-------------|
| `start` | Show how to start a session |
| `stop` | Show how to stop (Ctrl+C or kill) |
| `status` | Show how to check coordinator status |

---

## init

Initialize a new Pregel project scaffold. (Stub)

```bash
pregel init <NAME>
```

---

## Algorithms

| Name | Aliases | Description |
|------|---------|-------------|
| cc | connected_components | Connected components |
| pagerank | pr | PageRank |
| shortest_path | sssp, sp | Single-source shortest path (unweighted) |

---

## Port Layout (defaults)

| Component | Port | Purpose |
|-----------|------|---------|
| Coordinator gRPC | 5000 | Barrier sync, worker registration |
| Coordinator HTTP | 5100 | Job submission (session mode) |
| Worker 0 | 5001 | Inter-worker messaging |
| Worker 1 | 5002 | ... |
| Metrics (worker 0) | 9090 | /metrics (if --metrics-port 9090) |
| Metrics (worker 1) | 9091 | ... |

---

## Verbosity

| Level | Behavior |
|-------|----------|
| 0 | Quiet |
| 1 | Summary (superstep advances, halt) |
| 2 | Full dumps (messages, vertex states) |

Use `-v` or `--verbose` for level 1; `--verbose=2` for level 2.
