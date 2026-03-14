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
  aggregate(name: string, value: unknown): void;  // Stub; runtime not yet implemented
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
  superstep?: number;       // Default 0
  total_vertices?: number;   // Default 0
}

export interface ComputeResultWire {
  new_value: Uint8Array | null;
  outgoing: [VertexId, Uint8Array][];
}

/** Run a VertexProgram for local testing. Assumes u64 value/message encoding. */
export function runCompute<V = bigint, M = bigint>(
  program: VertexProgram<V, M>,
  input: ComputeInput,
  superstep = 0,
  valueSerde?: { serialize: (v: V) => Uint8Array; deserialize: (b: Uint8Array) => V },
  messageSerde?: { serialize: (m: M) => Uint8Array; deserialize: (b: Uint8Array) => M }
): ComputeResultWire {
  const deserV = valueSerde?.deserialize ?? ((b) => (b.length ? new DataView(b.buffer).getBigUint64(0, true) : BigInt(input.vertex_id)) as V);
  const serV = valueSerde?.serialize ?? ((v) => {
    const b = new ArrayBuffer(8);
    new DataView(b).setBigUint64(0, BigInt(v as bigint), true);
    return new Uint8Array(b);
  });
  const deserM = messageSerde?.deserialize ?? ((b) => (b.length ? new DataView(b.buffer).getBigUint64(0, true) : 0n) as M);
  const serM = messageSerde?.serialize ?? ((m) => {
    const b = new ArrayBuffer(8);
    new DataView(b).setBigUint64(0, BigInt(m as bigint), true);
    return new Uint8Array(b);
  });

  const value = input.value.length ? deserV(input.value) : (BigInt(input.vertex_id) as V);
  const vertex: Vertex<V> = { id: input.vertex_id, value, edges: [...input.edges] };
  const messages: Message<M>[] = input.messages.map(([src, p]) => [src, deserM(p)]);

  const outgoing: [VertexId, M][] = [];
  const totalVertices = input.total_vertices ?? 0;
  const ctx: Context<M> = {
    superstep,
    total_vertices: totalVertices,
    send(t, m) { outgoing.push([t, m]); },
    aggregate(_name, _value) { /* stub */ },
  };
  program.compute(vertex, messages, ctx);

  return {
    new_value: serV(vertex.value),
    outgoing: outgoing.map(([t, m]) => [t, serM(m)]),
  };
}
