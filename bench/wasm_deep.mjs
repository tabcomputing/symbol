import { readFileSync } from 'fs';
import { WASI } from 'wasi';
for (const depth of [1000, 10000, 100000]) {
  for (const path of process.argv.slice(2)) {
    const wasi = new WASI({ version: 'preview1', args: ['symbol'], env: {}, returnOnExit: true });
    const instance = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(path)), wasi.getImportObject());
    wasi.start(instance);
    const ex = instance.exports, enc = new TextEncoder(), dec = new TextDecoder();
    const run = (src) => {
      try {
        const bytes = enc.encode(src + '\0');
        const p = ex.symbol_alloc(bytes.length);
        new Uint8Array(ex.memory.buffer, p, bytes.length).set(bytes);
        const out = ex.symbol_eval(p);
        const mem = new Uint8Array(ex.memory.buffer); let e = out; while (mem[e]) e++;
        const s = dec.decode(mem.subarray(out, e)); ex.symbol_free(p); ex.symbol_free(out); return s.slice(0, 40);
      } catch (err) { return `TRAP (${err.constructor.name}: ${err.message})`; }
    };
    const src = '('.repeat(depth) + '1 + 1' + ')'.repeat(depth);
    const first = run(src), after = run('Σ [1, 2, 3, 4]');
    console.log(`depth ${String(depth).padStart(6)}  ${path.padEnd(22)} => ${first.padEnd(60)} then Σ => ${after}`);
  }
}
