# Pregel Go SDK

Implement `VertexProgram` to write graph algorithms in Go.

## Build / type check

```bash
cd sdk/go
go build ./...
```

## Usage

```go
type ConnectedComponents struct{}

func (c *ConnectedComponents) Compute(vertex *pregel.Vertex[uint64], messages []pregel.Message[uint64], ctx *pregel.Context[uint64]) {
    min := vertex.Value
    for _, m := range messages {
        if m.Payload < min {
            min = m.Payload
        }
    }
    vertex.Value = min
    for _, t := range vertex.Edges {
        ctx.Send(t, vertex.Value)
    }
}
```

## Building for WASM

```bash
# Standard Go
GOOS=js GOARCH=wasm go build -o algo.wasm .

# Or TinyGo (smaller output)
tinygo build -target=wasm -o algo.wasm .
```

The resulting `.wasm` must export `compute(input_ptr, input_len, output_ptr, output_max_len) -> i32`. Use the `//export compute` directive and implement the ABI in a `compute.go` that deserializes `ComputeInput`, runs your `VertexProgram`, serializes `ComputeResultWire`.

## ABI

See `docs/ABI_SPEC.md` for the wire format.
