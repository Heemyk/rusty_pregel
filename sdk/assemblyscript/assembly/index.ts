/**
 * Pregel vertex compute - AssemblyScript.
 * Build: npm run asbuild:release
 * Output: build/algo.release.wasm - use with pregel submit --program ...
 *
 * ABI: compute(input_ptr, input_len, output_ptr, output_max_len) -> output_len (or negative)
 */

const EINVALID = -1;
const EDESERIALIZE = -2;
const ESERIALIZE = -3;
const EOUTPUT_OVERRUN = -4;

function readU64(ptr: i32): u64 {
  return (
    (load<u8>(ptr) as u64) |
    ((load<u8>(ptr + 1) as u64) << 8) |
    ((load<u8>(ptr + 2) as u64) << 16) |
    ((load<u8>(ptr + 3) as u64) << 24) |
    ((load<u8>(ptr + 4) as u64) << 32) |
    ((load<u8>(ptr + 5) as u64) << 40) |
    ((load<u8>(ptr + 6) as u64) << 48) |
    ((load<u8>(ptr + 7) as u64) << 56)
  );
}

function writeU64(ptr: i32, v: u64): void {
  store<u8>(ptr, (v & 0xff) as u8);
  store<u8>(ptr + 1, ((v >> 8) & 0xff) as u8);
  store<u8>(ptr + 2, ((v >> 16) & 0xff) as u8);
  store<u8>(ptr + 3, ((v >> 24) & 0xff) as u8);
  store<u8>(ptr + 4, ((v >> 32) & 0xff) as u8);
  store<u8>(ptr + 5, ((v >> 40) & 0xff) as u8);
  store<u8>(ptr + 6, ((v >> 48) & 0xff) as u8);
  store<u8>(ptr + 7, ((v >> 56) & 0xff) as u8);
}

function copyMem(dst: i32, src: i32, len: i32): void {
  for (let i = 0; i < len; i++) store<u8>(dst + i, load<u8>(src + i));
}

/** Connected Components - bincode ComputeInput -> ComputeResultWire */
export function compute(
  inputPtr: i32,
  inputLen: i32,
  outputPtr: i32,
  outputMaxLen: i32
): i32 {
  if (inputLen <= 0 || outputMaxLen <= 0) return EINVALID;
  if (inputLen < 8) return EDESERIALIZE;

  let off = 0;
  const vertexId = readU64(inputPtr + off);
  off += 8;

  const valueLen = readU64(inputPtr + off);
  off += 8;
  let current = valueLen >= 8 ? readU64(inputPtr + off) : vertexId;
  off += (valueLen as i32);

  const edgesLen = readU64(inputPtr + off);
  off += 8;
  const edgesStart = off;
  off += (edgesLen * 8) as i32;

  const messagesLen = readU64(inputPtr + off);
  off += 8;

  const nMsg = messagesLen as i32;
  const nEdges = edgesLen as i32;

  let minReceived = current;
  let hasLargerSender = false;

  for (let i = 0; i < nMsg; i++) {
    const src = readU64(inputPtr + off);
    off += 8;
    const payLen = readU64(inputPtr + off);
    off += 8;
    if (payLen >= 8) {
      const v = readU64(inputPtr + off);
      if (v < minReceived) minReceived = v;
      if (v > current) hasLargerSender = true;
    }
    off += (payLen as i32);
    if (off > inputLen) return EDESERIALIZE;
  }

  // ABI extension: superstep, total_vertices (optional; skip if input is old format)
  if (off + 16 <= inputLen) {
    off += 16;
  }

  const newComp = current < minReceived ? current : minReceived;
  const shouldSend = !hasLargerSender
    ? newComp < current
    : true;

  let outOff = 0;
  store<u8>(outputPtr + outOff, 1);
  outOff += 1;
  writeU64(outputPtr + outOff, 8);
  outOff += 8;
  writeU64(outputPtr + outOff, newComp);
  outOff += 8;

  let outCount = 0;
  const tmp = outputPtr + 256;
  let tmpOff = 0;

  if (messagesLen == 0) {
    for (let i = 0; i < nEdges; i++) {
      const t = readU64(inputPtr + edgesStart + (i * 8));
      writeU64(tmp + tmpOff, t);
      tmpOff += 8;
      writeU64(tmp + tmpOff, 8);
      tmpOff += 8;
      writeU64(tmp + tmpOff, newComp);
      tmpOff += 8;
      outCount++;
    }
  } else if (shouldSend) {
    if (newComp < current) {
      for (let i = 0; i < nEdges; i++) {
        const t = readU64(inputPtr + edgesStart + (i * 8));
        writeU64(tmp + tmpOff, t);
        tmpOff += 8;
        writeU64(tmp + tmpOff, 8);
        tmpOff += 8;
        writeU64(tmp + tmpOff + 16, newComp);
        tmpOff += 24;
        outCount++;
      }
      let msgOff = 8 + 8 + (valueLen as i32) + 8 + (edgesLen * 8) as i32 + 8;
      for (let i = 0; i < nMsg; i++) {
        const src = readU64(inputPtr + msgOff);
        msgOff += 8;
        const payLen = readU64(inputPtr + msgOff);
        msgOff += 8;
        writeU64(tmp + tmpOff, src);
        tmpOff += 8;
        writeU64(tmp + tmpOff, 8);
        tmpOff += 8;
        writeU64(tmp + tmpOff + 16, newComp);
        tmpOff += 24;
        outCount++;
        msgOff += (payLen as i32);
      }
    } else {
      let msgOff2 = 8 + 8 + (valueLen as i32) + 8 + (edgesLen * 8) as i32 + 8;
      for (let i = 0; i < nMsg; i++) {
        const src = readU64(inputPtr + msgOff2);
        msgOff2 += 8;
        const payLen = readU64(inputPtr + msgOff2);
        msgOff2 += 8;
        if (payLen >= 8) {
          const v = readU64(inputPtr + msgOff2);
          if (v > current) {
            writeU64(tmp + tmpOff, src);
            tmpOff += 8;
            writeU64(tmp + tmpOff, 8);
            tmpOff += 8;
            writeU64(tmp + tmpOff + 16, newComp);
            tmpOff += 24;
            outCount++;
          }
        }
        msgOff2 += 8 + (payLen as i32);
      }
    }
  }

  writeU64(outputPtr + outOff, outCount);
  outOff += 8;
  copyMem(outputPtr + outOff, tmp, tmpOff);
  outOff += tmpOff;

  if (outOff > outputMaxLen) return EOUTPUT_OVERRUN;
  return outOff;
}
