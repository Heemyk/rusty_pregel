// Package pregel provides the Pregel SDK for Go.
// Implement VertexProgram to write graph algorithms.
//
// Build for WASM: GOOS=js GOARCH=wasm go build -o algo.wasm
// Or use tinygo: tinygo build -target=wasm -o algo.wasm .
package pregel

// VertexID is a unique vertex identifier.
type VertexID = uint64

// Vertex holds a vertex's id, value, and outgoing edges.
type Vertex[V any] struct {
	ID    VertexID
	Value V
	Edges []VertexID
}

// Context is passed to Compute; use Send to emit messages.
type Context[M any] struct {
	Superstep      uint64
	TotalVerticesN uint64 // avoids conflict with TotalVertices() method
	outgoing       []OutgoingMessage[M]
}

// OutgoingMessage is (target, payload).
type OutgoingMessage[M any] struct {
	Target VertexID
	Msg    M
}

// NewContext creates a context for the given superstep and total vertices.
func NewContext[M any](superstep, totalVertices uint64) *Context[M] {
	return &Context[M]{Superstep: superstep, TotalVerticesN: totalVertices, outgoing: nil}
}

// Send adds a message to be delivered at the next superstep.
func (c *Context[M]) Send(target VertexID, msg M) {
	c.outgoing = append(c.outgoing, OutgoingMessage[M]{Target: target, Msg: msg})
}

// TotalVertices returns the total number of vertices in the graph.
func (c *Context[M]) TotalVertices() uint64 {
	return c.TotalVerticesN
}

// Aggregate contributes to a named aggregator (stub; runtime not yet implemented).
func (c *Context[M]) Aggregate(name string, value any) {
	_, _ = name, value // TODO: runtime will collect and reduce
}
