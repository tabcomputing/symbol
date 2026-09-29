// Usage: node wasm_bench.mjs <module.wasm> <case> <iterations>
// Evaluates one case repeatedly through the C ABI (alloc input, eval, read
// output, free both), reporting linear-memory size and time.
import { readFile } from 'fs/promises';
import { WASI } from 'wasi';

const CASES = {
  sum:           ['symbol_eval',         'Σ [1, 2, 3, 4]', '10'],
  program:       ['symbol_eval_program', 'x = 3. y = x * 2. Σ [x, y, 10]', '19'],
  constraint:    ['symbol_eval_program', 'x = 3. y = 9. (x + y) > 10', 'true'],
  'grade+index': ['symbol_eval',         '(⍋ [30, 10, 20, 50, 40]) @> [30, 10, 20, 50, 40]', '[10, 20, 30, 40, 50]'],
  vector:        ['symbol_eval',         'Σ [1, 2, 3, 4, 5] * [2, 3, 4, 5, 6]', '70'],
  range:         ['symbol_eval',         'Σ 1 .. 100', '5050'],
};

const [path, caseName, n] = process.argv.slice(2);
const iterations = Number(n);
const [fn, source, expected] = CASES[caseName];

const wasi = new WASI({ version: 'preview1', args: ['symbol'], env: {}, returnOnExit: true });
const module = await WebAssembly.compile(await readFile(path));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
wasi.start(instance); // boots the Crystal runtime; a no-op for the Rust module
const ex = instance.exports;

const encoder = new TextEncoder(), decoder = new TextDecoder();
const input = encoder.encode(source + '\0');
function evalOnce() {
  const inPtr = ex.symbol_alloc(input.length);
  new Uint8Array(ex.memory.buffer, inPtr, input.length).set(input);
  const outPtr = ex[fn](inPtr);
  const mem = new Uint8Array(ex.memory.buffer);
  let end = outPtr;
  while (mem[end] !== 0) end++;
  const out = decoder.decode(mem.subarray(outPtr, end));
  ex.symbol_free(inPtr);
  ex.symbol_free(outPtr);
  return out;
}

const mb = () => (ex.memory.buffer.byteLength / 1048576).toFixed(2);
const checkpoints = new Set([1, 1000, 10000, iterations]);
const sizes = { before: mb() };
let failure = null, done = 0;
const start = process.hrtime.bigint();
try {
  for (let i = 1; i <= iterations; i++) {
    const out = evalOnce();
    if (out !== expected) throw new Error(`iteration ${i}: got ${JSON.stringify(out)}`);
    done = i;
    if (checkpoints.has(i)) sizes[i] = mb();
  }
} catch (e) {
  failure = `${e.message} after ${done} evals (memory ${mb()} MB)`;
}
const ms = Number(process.hrtime.bigint() - start) / 1e6;
console.log(JSON.stringify({ module: path.replace(/.*\//, ''), case: caseName, evals: done, ms: +ms.toFixed(1), memoryMB: sizes, failure }));
