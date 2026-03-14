# Pregel TypeScript SDK

Types and `runCompute()` for **local testing** of vertex algorithms in TypeScript.

## WASM: Use AssemblyScript

TypeScript compiles to JavaScript, not WASM. For production WASM, use the **AssemblyScript SDK** (`sdk/assemblyscript/`), which compiles TypeScript-like code to WASM. See `sdk/assemblyscript/README.md`.

## Local Testing

Use `runCompute(program, input)` to test algorithms without the full runtime:

```typescript
import { VertexProgram, runCompute, ComputeInput } from "pregel-sdk";

const CC: VertexProgram<bigint, bigint> = {
  compute(vertex, messages, ctx) {
    const min = messages.reduce((m, [, p]) => (p < m ? p : m), vertex.value);
    vertex.value = vertex.value < min ? vertex.value : min;
    for (const t of vertex.edges) ctx.send(t, vertex.value);
  },
};
const out = runCompute(CC, input);
```

## ABI

See `docs/ABI_SPEC.md`.
