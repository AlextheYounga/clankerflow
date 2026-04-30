import { test } from "node:test";
import assert from "node:assert";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";

import { createFsContext } from "../src/tools/fs.ts";

test("FsContext: workspace scoping", async (t) => {
  const tmpDir = await fs.mkdtemp(
    path.join(os.tmpdir(), "clankerflow-fs-test-")
  );
  const fsCtx = createFsContext(tmpDir);

  await t.test("should write and read files", async () => {
    await fsCtx.write("test.txt", "hello world");
    const content = await fsCtx.read("test.txt");
    assert.strictEqual(content, "hello world");
  });

  await t.test("should support 'read' alias for compatibility", async () => {
    const content = await fsCtx.read("test.txt");
    assert.strictEqual(content, "hello world");
  });

  await t.test("should check for existence", async () => {
    const exists = await fsCtx.exists("test.txt");
    assert.strictEqual(exists, true);
    const notExists = await fsCtx.exists("missing.txt");
    assert.strictEqual(notExists, false);
  });

  await t.test("should prevent path escapes", async () => {
    await assert.rejects(
      () => fsCtx.read("../escaped.txt"),
      /escapes workspace root/
    );
  });

  await fs.rm(tmpDir, { recursive: true });
});
