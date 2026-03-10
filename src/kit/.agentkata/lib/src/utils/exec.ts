import { spawn } from "node:child_process";
import path from "node:path";

import type { RuntimeEnv } from "../protocol.ts";

export function runExec(
  command: string,
  args: string[],
  cwd: string,
  signal: AbortSignal
): Promise<{ code: number; stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      stdout += chunk;
    });

    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk: string) => {
      stderr += chunk;
    });

    const onAbort = () => {
      child.kill("SIGTERM");
      reject(new Error("operation cancelled"));
    };
    signal.addEventListener("abort", onAbort, { once: true });

    child.on("error", (error) => {
      signal.removeEventListener("abort", onAbort);
      reject(error);
    });
    child.on("close", (code) => {
      signal.removeEventListener("abort", onAbort);
      // Non-zero exit codes are returned to workflow code instead of throwing so
      // workflows can make explicit policy decisions per command.
      resolve({ code: code ?? 0, stdout, stderr });
    });
  });
}

export function resolveExecSpec(
  runtimeEnv: RuntimeEnv,
  command: string,
  args: string[],
  workspaceRoot: string
): { bin: string; args: string[]; cwd: string } {
  if (runtimeEnv === "host") {
    return { bin: command, args, cwd: workspaceRoot };
  }

  // Container mode shells out through `docker compose exec` so workflows can
  // keep using the same `ctx.exec` API regardless of runtime target.
  return {
    bin: "docker",
    args: [
      "compose",
      "-f",
      path.join(
        workspaceRoot,
        ".agents",
        ".agentkata",
        "docker",
        "agent.docker-compose.yaml"
      ),
      "exec",
      // Disable pseudo-TTY to keep output deterministic for programmatic parsing.
      "-T",
      "agent",
      command,
      ...args,
    ],
    cwd: workspaceRoot,
  };
}
