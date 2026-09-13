#!/usr/bin/env node
/** Copy the NSIS output to T-Books-Setup.exe without deleting the original. */
import { copyFileSync, existsSync, mkdirSync, readdirSync, realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SETUP_NAME = "T-Books-Setup.exe";

export function nsisBundleDirs(root) {
  return [
    join(root, "src-tauri", "target", "release", "bundle", "nsis"),
    join(root, "src-tauri", "target", "x86_64-pc-windows-msvc", "release", "bundle", "nsis"),
  ];
}

/** Tauri writes `{product}_{version}_{arch}-setup.exe`. Never pick WebView2/Edge. */
export function pickSetupExe(files) {
  const names = files.filter((f) => typeof f === "string" && f.toLowerCase().endsWith("-setup.exe"));
  const safe = names.filter((f) => {
    const l = f.toLowerCase();
    if (l.includes("credentials")) return false;
    if (l.includes("webview")) return false;
    if (l.includes("edge")) return false;
    return true;
  });
  const branded = safe.filter((f) => {
    const l = f.toLowerCase();
    return l.includes("t books") || l.includes("t-books") || l.startsWith("t_books");
  });
  return branded[0] ?? safe[0] ?? null;
}

function isMainModule() {
  const entry = process.argv[1];
  if (!entry) return false;
  try {
    return realpathSync(entry) === fileURLToPath(import.meta.url);
  } catch {
    return false;
  }
}

function main() {
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const dirs = nsisBundleDirs(root).filter((dir) => existsSync(dir));
  if (dirs.length === 0) {
    console.error("No NSIS bundle yet.");
    process.exit(1);
  }
  let found = null;
  let fromDir = null;
  for (const dir of dirs) {
    const pick = pickSetupExe(readdirSync(dir));
    if (pick) {
      found = pick;
      fromDir = dir;
      break;
    }
  }
  if (!found || !fromDir) {
    console.error("No setup exe in the NSIS folder.");
    process.exit(1);
  }
  const outDir = join(root, "dist", "installers");
  mkdirSync(outDir, { recursive: true });
  const dest = join(outDir, SETUP_NAME);
  copyFileSync(join(fromDir, found), dest);
  console.log(dest);
}

if (isMainModule()) {
  main();
}
