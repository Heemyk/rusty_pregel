/**
 * Quick test for TypeScript SDK runCompute with Connected Components.
 * Run: npx ts-node test-cc.ts   (or: npm run build && node -e "require('./dist')...")
 */
import { runCompute, ComputeInput } from ".";

const CC = {
  compute(
    vertex: { id: number; value: bigint; edges: number[] },
    messages: [number, bigint][],
    ctx: { send: (t: number, m: bigint) => void }
  ) {
    const min = messages.reduce((m, [, p]) => (p < m ? p : m), vertex.value);
    vertex.value = vertex.value < min ? vertex.value : min;
    for (const t of vertex.edges) ctx.send(t, vertex.value);
  },
};

function buf(n: number): Uint8Array {
  const b = new ArrayBuffer(8);
  new DataView(b).setBigUint64(0, BigInt(n), true);
  return new Uint8Array(b);
}

// Superstep 0: vertex 5, edges to 1,2,3
const input0: ComputeInput = {
  vertex_id: 5,
  value: buf(5),
  edges: [1, 2, 3],
  messages: [],
};
const out0 = runCompute(CC, input0, 0);
console.log("Superstep 0:", new DataView(out0.new_value!.buffer).getBigUint64(0, true), "outgoing:", out0.outgoing.length);
if (out0.outgoing.length !== 3) throw new Error("Expected 3 outgoing");

// Superstep 1: vertex 5 receives 1,2 from neighbors
const input1: ComputeInput = {
  vertex_id: 5,
  value: buf(5),
  edges: [1, 2, 3],
  messages: [[1, buf(1)], [2, buf(2)]],
};
const out1 = runCompute(CC, input1, 1);
const comp = Number(new DataView(out1.new_value!.buffer).getBigUint64(0, true));
console.log("Superstep 1: vertex 5 -> component", comp);
if (comp !== 1) throw new Error("Expected component 1, got " + comp);

console.log("TypeScript SDK: OK");
