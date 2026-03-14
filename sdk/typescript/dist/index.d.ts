/**
 * Pregel TypeScript SDK — implement VertexProgram to write graph algorithms.
 *
 * For WASM: implement the same logic in Rust and use `pregel build`.
 * For local testing: use runCompute(program, input).
 */
export type VertexId = number;
export interface Vertex<V> {
    id: VertexId;
    value: V;
    edges: VertexId[];
}
export interface Context<M> {
    superstep: number;
    total_vertices: number;
    send(target: VertexId, msg: M): void;
    aggregate(name: string, value: unknown): void;
}
/** (source, payload) */
export type Message<M> = [VertexId, M];
export interface VertexProgram<V, M> {
    compute(vertex: Vertex<V>, messages: Message<M>[], ctx: Context<M>): void;
}
export interface ComputeInput {
    vertex_id: VertexId;
    value: Uint8Array;
    edges: VertexId[];
    messages: [VertexId, Uint8Array][];
    superstep?: number;
    total_vertices?: number;
}
export interface ComputeResultWire {
    new_value: Uint8Array | null;
    outgoing: [VertexId, Uint8Array][];
}
/** Run a VertexProgram for local testing. Assumes u64 value/message encoding. */
export declare function runCompute<V = bigint, M = bigint>(program: VertexProgram<V, M>, input: ComputeInput, superstep?: number, valueSerde?: {
    serialize: (v: V) => Uint8Array;
    deserialize: (b: Uint8Array) => V;
}, messageSerde?: {
    serialize: (m: M) => Uint8Array;
    deserialize: (b: Uint8Array) => M;
}): ComputeResultWire;
