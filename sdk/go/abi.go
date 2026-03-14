// Package pregel ABI types. See docs/ABI_SPEC.md.

package pregel

// ComputeInput is the wire-format input for vertex compute.
type ComputeInput struct {
	VertexID       VertexID
	Value          []byte
	Edges          []VertexID
	Messages       []MessagePayload
	Superstep      uint64 // Current superstep (0-indexed)
	TotalVertices  uint64 // Total vertices in graph (e.g. PageRank 1/N)
}

// MessagePayload is (source, serialized payload).
type MessagePayload struct {
	Source  VertexID
	Payload []byte
}

// ComputeResultWire is the wire-format output.
type ComputeResultWire struct {
	NewValue []byte       // nil = vote to halt
	Outgoing []TargetPayload
}

// TargetPayload is (target, serialized message).
type TargetPayload struct {
	Target  VertexID
	Payload []byte
}

// AbiErrorCode matches docs/ABI_SPEC.md §2.2.
const (
	AbiErrInvalid       = -1
	AbiErrDeserialize   = -2
	AbiErrSerialize     = -3
	AbiErrOutputOverrun = -4
	AbiErrAlloc         = -5
	AbiErrUser          = -6
)
