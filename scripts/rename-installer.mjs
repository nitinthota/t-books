#!/usr/bin/env node
/** Copy the NSIS output to LoopBooks-Setup.exe without deleting the original. */
import { copyFileSync, existsSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const nsis = join(root, "src-tauri", "target", "release", "bundle", "nsis");
const outDir = join(root, "dist", "installers");
if (!existsSync(nsis)) {
  console.error("No NSIS bundle yet.");
  process.exit(1);
}
const found = readdirSync(nsis).find((f) => f.toLowerCase().endsWith("-setup.exe") || f.toLowerCase().endsWith("setup.exe"));
if (!found) {
  console.error("No setup exe in the NSIS folder.");
  process.exit(1);
}
mkdirSync(outDir, { recursive: true });
if (found.toLowerCase().includes("credentials")) {
  console.error("Refusing to copy a credentials-named file as the installer.");
  process.exit(1);
}
const dest = join(outDir, "LoopBooks-Setup.exe");
copyFileSync(join(nsis, found), dest);
console.log(dest);
