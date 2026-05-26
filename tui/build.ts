import solidPlugin from "@opentui/solid/bun-plugin";
import { copyFile, mkdir } from "node:fs/promises";

const result = await Bun.build({
  entrypoints: ["./src/index.tsx"],
  target: "bun",
  outdir: "./dist",
  plugins: [solidPlugin],
});

if (!result.success) {
  for (const log of result.logs) console.error(log);
  process.exit(1);
}

await mkdir("./dist", { recursive: true });
await copyFile("./node_modules/@jsquash/webp/codec/dec/webp_dec.wasm", "./dist/webp_dec.wasm");
