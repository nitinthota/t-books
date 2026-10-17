#!/usr/bin/env node
/** Fail the build if the Tauri web UI was not produced. */
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const root = join(process.cwd(), "dist", "client");
if (!existsSync(root)) {
  console.error("dist/client missing — vite.tauri build did not run.");
  process.exit(1);
}
const index = join(root, "index.html");
if (!existsSync(index)) {
  console.error("dist/client/index.html missing — UI shell not packaged.");
  process.exit(1);
}
const html = readFileSync(index, "utf8");
if (!html.includes("app") || html.length < 80) {
  console.error("dist/client/index.html looks empty.");
  process.exit(1);
}

function walk(dir) {
  let total = 0;
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) total += walk(p);
    else total += st.size;
  }
  return total;
}
const bytes = walk(root);
const files = readdirSync(root);
console.log(`UI bundle: ${files.length} top-level entries, ${bytes} bytes under dist/client`);
// Real React+Tailwind build for this app is well above 50 KB.
if (bytes < 50_000) {
  console.error(
    `dist/client is only ${bytes} bytes — UI assets were not bundled. Abort packaging.`,
  );
  process.exit(1);
}
console.log("UI package OK");
