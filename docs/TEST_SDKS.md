# Testing the Pregel SDKs

Quick commands to verify each SDK works.

## Prerequisites

- Rust: `cargo`
- Node: `npm`, `node`
- AssemblyScript: via `npm install` in `sdk/assemblyscript`
- Go (optional): `go` for type-checking `sdk/go`

---

## 1. Rust SDK

```bash
cd pregel
cargo run -p connected-components-example
```

**Expected:** `Adapter test passed: vertex 5 updated to component 1`

---

## 2. AssemblyScript SDK

```bash
cd pregel/sdk/assemblyscript
npm install
npm run asbuild:release
```

**Expected:** `build/algo.release.wasm` created (~3KB)

**Run with pregel:**
```bash
cd ../..
cargo run -p pregel-cli -- submit --graph examples/data/sample.graph \
  --algo cc --program sdk/assemblyscript/build/algo.release.wasm
```

*(Use `--coordinator-port 6000` if 5000 is in use.)*

---

## 3. TypeScript SDK

```bash
cd pregel/sdk/typescript
npm install
npm run test:cc
```

**Expected:** `TypeScript SDK: OK (vertex 5 -> component 1)`

---

## 4. Go SDK

```bash
cd pregel/sdk/go
go build ./...
```

**Expected:** Compiles (types only; no full `compute` export yet).

---

## One-liner script

From `pregel/`:

```bash
./scripts/test-sdks.sh
```
