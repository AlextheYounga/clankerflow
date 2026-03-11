import test from "node:test";
import assert from "node:assert/strict";

import { createContext, resolveExecSpec } from "../src/context.ts";
import { createExec } from "../src/utils.ts";

test("resolveExecSpec keeps host commands unchanged", () => {
  const spec = resolveExecSpec(
    "host",
    "git",
    ["status", "--short"],
    "/workspace"
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
    "/workspace/.agents/.agentkata/docker/agent.docker-compose.yaml",
    "exec",
    "-T",
    "agent",
    "git",
    "status",
  ]);
});

test("createExec runs host command", async () => {
  const controller = new AbortController();
  const exec = createExec({
    workspaceRoot: process.cwd(),
    runtimeEnv: "host",
    signal: controller.signal,
  });

  const result = await exec("node", ["-e", "process.stdout.write('ok')"]);

  assert.equal(result.code, 0);
  assert.equal(result.stdout, "ok");
});

test("createContext returns run metadata without tools", () => {
  const controller = new AbortController();
  const context = createContext({
    workspaceRoot: "/workspace",
    runtimeEnv: "container",
    yolo: true,
    signal: controller.signal,
    ticket: { id: "T-1" },
  });

  assert.equal(context.workspaceRoot, "/workspace");
  assert.equal(context.runtimeEnv, "container");
  assert.equal(context.yolo, true);
  assert.equal(context.ticket, null);
  assert.equal(context.signal, controller.signal);
});

test("createContext preserves valid ticket shape", () => {
  const controller = new AbortController();
  const ticket = {
    ticketId: "T-1",
    title: "Title",
    status: "OPEN",
    worktree: "main",
    description: "Details",
    filePath: ".agents/tickets/T-1.md",
    frontmatter: { id: "T-1" },
  } as const;

  const context = createContext({
    workspaceRoot: "/workspace",
    runtimeEnv: "host",
    yolo: false,
    signal: controller.signal,
    ticket,
  });

  assert.deepEqual(context.ticket, ticket);
});
