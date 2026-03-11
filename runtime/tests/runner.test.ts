import test from "node:test";
import assert from "node:assert/strict";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { once } from "node:events";

interface IpcMessage {
  v: "v1";
  id: string;
  kind: "command" | "event" | "response" | "error";
  name: string;
  payload: Record<string, unknown>;
}

const filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(filename);

test("runner emits deterministic lifecycle events for successful run", async () => {
  const workflowPath = path.join(dirname, "fixtures/workflow-valid.ts");
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
    runId
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
  const workflowPath = path.join(dirname, "fixtures/workflow-cancel.ts");
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
    runId
  );

  const finish = events.find((event) => event.name === "run_finished");
  assert.ok(finish);
  assert.equal(finish.payload.status, "CANCELLED");
});

function command(
  id: string,
  name: string,
  payload: Record<string, unknown>
): IpcMessage {
  return { v: "v1", id, kind: "command", name, payload };
}

async function runRunnerSequence(
  commands: IpcMessage[],
  runId: number
): Promise<IpcMessage[]> {
  const runnerPath = path.join(dirname, "../src/runner.ts");

  const server = net.createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = server.address() as net.AddressInfo;
  const port = address.port;

  const child = spawn("node", [runnerPath], {
    cwd: path.join(dirname, ".."),
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env, AGENTCTL_IPC_PORT: String(port) },
  });
  const closePromise = once(child, "close");

  const [socket] = (await once(server, "connection")) as [net.Socket];

  const events: IpcMessage[] = [];
  let buffer = "";
  socket.on("data", (chunk) => {
    buffer += chunk.toString();
    const lines = buffer.split("\n");
    buffer = lines.pop()!;
    for (const line of lines) {
      if (line.trim()) {
        const message = JSON.parse(line) as IpcMessage;
        if (message.kind === "event") {
          events.push(message);
        }
      }
    }
  });

  // Drain stderr to prevent blocking; we don't assert on it in these tests.
  assert.ok(child.stderr);
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (_chunk: string) => {
    /* drain */
  });

  for (const message of commands) {
    socket.write(JSON.stringify(message) + "\n");
    await delay(20);
  }

  await waitFor(() =>
    events.some(
      (event) => event.name === "run_finished" && event.payload.run_id === runId
    )
  );

  socket.write(
    JSON.stringify(
      command("shutdown", "shutdown", { reason: "test_complete" })
    ) + "\n"
  );
  await delay(20);

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
  if (timeoutHandle !== undefined) {
    clearTimeout(timeoutHandle);
  }

  socket.destroy();
  server.close();

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
