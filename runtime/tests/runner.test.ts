import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { once } from "node:events";

type IpcMessage = {
  v: "v1";
  id: string;
  kind: "command" | "event" | "response" | "error";
  name: string;
  payload: Record<string, unknown>;
};

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test("runner emits deterministic lifecycle events for successful run", async () => {
  const workflowPath = path.join(__dirname, "fixtures/workflow-valid.ts");
  const runId = 101;
  const events = await runRunnerSequence(
    [
      command("1", "start_run", {
        run_id: runId,
        workflow_path: workflowPath,
        runtime_env: "host",
        yolo: false,
        workflow_input: {},
      }),
    ],
    runId,
  );

  const names = events.map((event) => event.name);
  assert.deepEqual(names, [
    "run_started",
    "step_started",
    "log",
    "log",
    "step_finished",
    "run_finished",
  ]);
  const finish = events.find((event) => event.name === "run_finished");
  assert.equal(finish?.payload.status, "SUCCEEDED");
});

test("runner cancels active run and finishes with CANCELLED", async () => {
  const workflowPath = path.join(__dirname, "fixtures/workflow-cancel.ts");
  const runId = 202;
  const events = await runRunnerSequence(
    [
      command("1", "start_run", {
        run_id: runId,
        workflow_path: workflowPath,
        runtime_env: "host",
        yolo: false,
        workflow_input: {},
      }),
      command("2", "cancel_run", {
        run_id: runId,
        reason: "user_requested",
      }),
    ],
    runId,
  );

  const finish = events.find((event) => event.name === "run_finished");
  assert.ok(finish);
  assert.equal(finish?.payload.status, "CANCELLED");
});

function command(
  id: string,
  name: string,
  payload: Record<string, unknown>,
): IpcMessage {
  return { v: "v1", id, kind: "command", name, payload };
}

async function runRunnerSequence(
  commands: IpcMessage[],
  runId: number,
): Promise<IpcMessage[]> {
  const runnerPath = path.join(__dirname, "../src/runner.ts");
  const child = spawn("node", [runnerPath], {
    cwd: path.join(__dirname, ".."),
    stdio: ["ignore", "pipe", "pipe", "ipc"],
  });
  const closePromise = once(child, "close");

  const events: IpcMessage[] = [];
  child.on("message", (raw: unknown) => {
    const message = raw as IpcMessage;
    if (message?.kind === "event") {
      events.push(message);
    }
  });

  child.stderr?.setEncoding("utf8");
  child.stderr?.on("data", () => {});

  for (const message of commands) {
    child.send?.(message);
    await delay(20);
  }

  await waitFor(() =>
    events.some(
      (event) =>
        event.name === "run_finished" && event.payload.run_id === runId,
    ),
  );

  child.send?.(command("shutdown", "shutdown", { reason: "test_complete" }));
  await delay(20);
  if (child.connected) {
    child.disconnect();
  }

  let timeoutHandle: NodeJS.Timeout | undefined;
  const closeResult = await Promise.race([
    closePromise,
    new Promise<never>((_, reject) => {
      timeoutHandle = setTimeout(() => {
        child.kill("SIGKILL");
        reject(new Error("runner process did not exit within timeout"));
      }, 5_000);
    }),
  ]);
  if (timeoutHandle) {
    clearTimeout(timeoutHandle);
  }

  const [exitCode] = closeResult as [number];
  assert.equal(exitCode, 0);

  return events;
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitFor(check: () => boolean): Promise<void> {
  const deadline = Date.now() + 5_000;
  while (Date.now() < deadline) {
    if (check()) {
      return;
    }
    await delay(20);
  }
  throw new Error("timed out waiting for expected runner event");
}
