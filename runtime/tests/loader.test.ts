import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { loadWorkflowModule } from "../src/loader.ts";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test("loadWorkflowModule accepts valid metadata and async default", async () => {
  const workflowPath = path.join(__dirname, "fixtures/workflow-valid.ts");
  const loaded = await loadWorkflowModule(workflowPath);

  assert.equal(loaded.meta.id, "duos");
  assert.equal(loaded.meta.runtime, "host");
  assert.equal(typeof loaded.run, "function");
});

test("loadWorkflowModule rejects invalid metadata", async () => {
  const workflowPath = path.join(
    __dirname,
    "fixtures/workflow-invalid-meta.ts",
  );

  await assert.rejects(
    loadWorkflowModule(workflowPath),
    /workflow meta.id must be a non-empty string/,
  );
});

test("loadWorkflowModule rejects non-async default export", async () => {
  const workflowPath = path.join(
    __dirname,
    "fixtures/workflow-invalid-default.ts",
  );

  await assert.rejects(
    loadWorkflowModule(workflowPath),
    /default export must be an async function/,
  );
});
