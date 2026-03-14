"use strict";
/**
 * Pregel TypeScript SDK — implement VertexProgram to write graph algorithms.
 *
 * For WASM: implement the same logic in Rust and use `pregel build`.
 * For local testing: use runCompute(program, input).
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.runCompute = runCompute;
/** Run a VertexProgram for local testing. Assumes u64 value/message encoding. */
function runCompute(program, input, superstep = 0, valueSerde, messageSerde) {
    const deserV = valueSerde?.deserialize ?? ((b) => (b.length ? new DataView(b.buffer).getBigUint64(0, true) : BigInt(input.vertex_id)));
    const serV = valueSerde?.serialize ?? ((v) => {
        const b = new ArrayBuffer(8);
        new DataView(b).setBigUint64(0, BigInt(v), true);
        return new Uint8Array(b);
    });
    const deserM = messageSerde?.deserialize ?? ((b) => (b.length ? new DataView(b.buffer).getBigUint64(0, true) : 0n));
    const serM = messageSerde?.serialize ?? ((m) => {
        const b = new ArrayBuffer(8);
        new DataView(b).setBigUint64(0, BigInt(m), true);
        return new Uint8Array(b);
    });
    const value = input.value.length ? deserV(input.value) : BigInt(input.vertex_id);
    const vertex = { id: input.vertex_id, value, edges: [...input.edges] };
    const messages = input.messages.map(([src, p]) => [src, deserM(p)]);
    const outgoing = [];
    const totalVertices = input.total_vertices ?? 0;
    const ctx = {
        superstep,
        total_vertices: totalVertices,
        send(t, m) { outgoing.push([t, m]); },
        aggregate(_name, _value) { },
    };
    program.compute(vertex, messages, ctx);
    return {
        new_value: serV(vertex.value),
        outgoing: outgoing.map(([t, m]) => [t, serM(m)]),
    };
}
