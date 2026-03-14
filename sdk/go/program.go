package pregel

// VertexProgram is the interface for vertex compute.
// Implement Compute; messages are (sourceID, payload) pairs.
type VertexProgram[V, M any] interface {
	Compute(vertex *Vertex[V], messages []Message[M], ctx *Context[M])
}

// Message is (source_vertex_id, payload).
type Message[M any] struct {
	Source VertexID
	Payload M
}
