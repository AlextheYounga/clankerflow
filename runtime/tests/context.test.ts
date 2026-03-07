import test from "node:test";
import assert from "node:assert/strict";

import { createContext, resolveExecSpec } from "../src/context.ts";

test("resolveExecSpec keeps host commands unchanged", () => {
  const spec = resolveExecSpec(
    "host",
    "git",
    ["status", "--short"],
    "/workspace",
  );

  assert.equal(spec.bin, "git");
  assert.deepEqual(spec.args, ["status", "--short"]);
  assert.equal(spec.cwd, "/workspace");
});

test("resolveExecSpec maps container commands to docker compose exec", () => {
  const spec = resolveExecSpec("container", "git", ["status"], "/workspace");

  assert.equal(spec.bin, "docker");
  assert.equal(spec.cwd, "/workspace");
  assert.deepEqual(spec.args, [
    "compose",
    "-f",
    "/workspace/.agents/.agentctl/docker/agent.docker-compose.yaml",
    "exec",
    "-T",
    "agent",
    "git",
    "status",
  ]);
});

test("createContext exec runs host command", async () => {
  const controller = new AbortController();
  const context = createContext({
    workspaceRoot: process.cwd(),
    runtimeEnv: "host",
    yolo: false,
    signal: controller.signal,
    emitEvent: () => {},
    invokeCapability: async () => ({}),
  });

  const result = await context.exec("node", ["-e", "process.stdout.write('ok')"]);

  assert.equal(result.code, 0);
  assert.equal(result.stdout, "ok");
});
