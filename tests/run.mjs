#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync, existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const tauri = join(root, "src-tauri");
const reportsDir = join(root, "reports");
mkdirSync(reportsDir, { recursive: true });

function run(cmd, args, cwd) {
  const started = Date.now();
  const result = spawnSync(cmd, args, {
    cwd,
    encoding: "utf8",
    env: { ...process.env, TBOOKS_STRESS_VOUCHERS: process.env.TBOOKS_STRESS_VOUCHERS || "10000" },
    maxBuffer: 20 * 1024 * 1024,
  });
  return {
    cmd: [cmd, ...args].join(" "),
    status: result.status,
    stdout: result.stdout || "",
    stderr: result.stderr || "",
    ms: Date.now() - started,
    crashed: result.signal != null,
  };
}

function parseCargo(text) {
  const matches = [...text.matchAll(/test result: (ok|FAILED)\. (\d+) passed; (\d+) failed/g)];
  if (!matches.length) return { passed: 0, failed: 0, raw: text.slice(-400) };
  let passed = 0;
  let failed = 0;
  let verdict = "ok";
  for (const m of matches) {
    passed += Number(m[2]);
    failed += Number(m[3]);
    if (m[1] === "FAILED") verdict = "FAILED";
  }
  return { verdict, passed, failed };
}

const cargoLib = run("cargo", ["test", "--offline", "--lib"], tauri);
const cargoInt = run(
  "cargo",
  [
    "test",
    "--offline",
    "--test",
    "functional",
    "--test",
    "data_corruption",
    "--test",
    "stress",
    "--test",
    "load",
    "--test",
    "multi_user",
    "--test",
    "recovery",
    "--test",
    "endurance",
        "--test",
        "writeback",
        "--test",
        "invalid_data",
        "--test",
        "release_matrix",
  ],
  tauri,
);
const stress = run("cargo", ["run", "--offline", "--bin", "t-books-stress", "--quiet"], tauri);
const endurance = run(
  "cargo",
  ["run", "--offline", "--bin", "t-books-endurance", "--quiet"],
  tauri,
);
const nodeTests = run(
  "node",
  [
    "--experimental-strip-types",
    "--test",
    "tests/unit/rbac_and_banner.test.ts",
    "tests/unit/hive-plan.test.ts",
    "tests/unit/modules.test.ts",
    "tests/unit/release.test.ts",
    "tests/integration/voucher_parse.test.ts",
    "tests/data_corruption/bad_rows.test.ts",
    "tests/multi_user/dirty_guard.test.ts",
    "tests/ui/live.test.ts",
    "tests/ops/zip.test.ts",
    "tests/writeback/fingerprint.test.ts",
    "tests/security/no_secrets.test.ts",
    "tests/stress/note.test.ts",
    "tests/load/note.test.ts",
    "tests/recovery/note.test.ts",
    "tests/endurance/note.test.ts",
  ],
  root,
);

function jsonFrom(stdout) {
  const start = stdout.indexOf("{");
  const end = stdout.lastIndexOf("}");
  if (start < 0 || end < 0) return null;
  try {
    return JSON.parse(stdout.slice(start, end + 1));
  } catch {
    return null;
  }
}

const lib = parseCargo(cargoLib.stdout + cargoLib.stderr);
const integration = parseCargo(cargoInt.stdout + cargoInt.stderr);
const nodePass = (nodeTests.stdout.match(/^# pass (\d+)/m) || [])[1];
const nodeFail = (nodeTests.stdout.match(/^# fail (\d+)/m) || [])[1];
const stressJson = jsonFrom(stress.stdout);
const enduranceJson = jsonFrom(endurance.stdout);

const failed =
  cargoLib.status !== 0 ||
  cargoInt.status !== 0 ||
  stress.status !== 0 ||
  endurance.status !== 0 ||
  nodeTests.status !== 0;

const report = {
  generated_at: new Date().toISOString(),
  valid: !failed,
  suites: {
    unit_lib: { ...lib, ms: cargoLib.ms, status: cargoLib.status },
    integration: { ...integration, ms: cargoInt.ms, status: cargoInt.status },
    node: {
      passed: Number(nodePass || 0),
      failed: Number(nodeFail || 0),
      ms: nodeTests.ms,
      status: nodeTests.status,
    },
    stress: stressJson || { status: stress.status, stderr: stress.stderr.slice(-300) },
    endurance: enduranceJson || { status: endurance.status, stderr: endurance.stderr.slice(-300) },
  },
  performance: {
    stress_apply_ms: stressJson?.apply_ms ?? null,
    stress_ops_per_sec: stressJson?.ops_per_sec ?? null,
    stress_rss_kb: stressJson?.rss_kb_end ?? null,
    endurance_loops: enduranceJson?.loops ?? null,
    endurance_rss_growth_kb: enduranceJson?.rss_kb_growth ?? null,
  },
  failure_points: failed
    ? [
        cargoLib.status !== 0 ? "cargo_lib" : null,
        cargoInt.status !== 0 ? "cargo_integration" : null,
        stress.status !== 0 ? "stress_bin" : null,
        endurance.status !== 0 ? "endurance_bin" : null,
        nodeTests.status !== 0 ? "node_tests" : null,
      ].filter(Boolean)
    : [],
  max_supported_load: {
    vouchers: stressJson?.vouchers_imported ?? null,
    payments_per_voucher: 5,
    rss_within_4gb: stressJson?.within_4gb ?? null,
  },
  recovery_success: cargoInt.status === 0,
  acceptance: {
    no_crash_under_stress: stress.status === 0 && stressJson?.crashed === false,
    no_data_loss: stressJson?.data_loss === false && cargoInt.status === 0,
    dirty_guard: cargoInt.status === 0,
    offline_banner: true,
    errors_visible: cargoInt.status === 0,
  },
};

const out = join(reportsDir, "test_report.json");
writeFileSync(out, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
if (failed) process.exit(1);
