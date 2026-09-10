import assert from "node:assert/strict";
import { test } from "node:test";
import { unzipNamed, zipStore } from "../../src/lib/t-books/zip.ts";

test("zip store roundtrip tbooks.db", () => {
  const payload = new TextEncoder().encode("sqlite-bytes");
  const zip = zipStore([
    { name: "tbooks.db", data: payload },
    { name: "manifest.json", data: new TextEncoder().encode("{}") },
  ]);
  const out = unzipNamed(zip, "tbooks.db");
  assert.ok(out);
  assert.equal(new TextDecoder().decode(out), "sqlite-bytes");
});

test("missing db is null not throw", () => {
  const zip = zipStore([{ name: "readme.txt", data: new Uint8Array([1]) }]);
  assert.equal(unzipNamed(zip, "tbooks.db"), null);
});
