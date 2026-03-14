/**
 * Test TypeScript SDK runCompute (run after: npm run build)
 * node test-cc.cjs.js
 */
const { runCompute } = require("./dist/index.js");

const CC = {
  compute(vertex, messages, ctx) {
    const min = messages.reduce((m, [, p]) => (p < m ? p : m), vertex.value);
    vertex.value = vertex.value < min ? vertex.value : min;
    for (const t of vertex.edges) ctx.send(t, vertex.value);
  },
};

function buf(n) {
  const b = new ArrayBuffer(8);
  new DataView(b).setBigUint64(0, BigInt(n), true);
  return new Uint8Array(b);
}

const input = {
  vertex_id: 5,
  value: buf(5),
  edges: [1, 2, 3],
  messages: [[1, buf(1)], [2, buf(2)]],
};
const out = runCompute(CC, input, 1);
const comp = new DataView(out.new_value.buffer).getBigUint64(0, true);
if (comp !== 1n) throw new Error("Expected 1, got " + comp);
console.log("TypeScript SDK: OK (vertex 5 -> component 1)");
