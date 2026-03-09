import { spawn } from "node:child_process";

import { createFsContext } from "./helpers/fs.ts";
import { createGitContext } from "./helpers/git.ts";
import { createTicketContext } from "./helpers/tickets/index.ts";

const workspaceRoot = process.cwd();

export const fs = createFsContext(workspaceRoot);
export const git = createGitContext(workspaceRoot);
export const tickets = createTicketContext(workspaceRoot);

export function sleepWithSignal(
  ms: number,
  signal: AbortSignal,
): Promise<void> {
  if (signal.aborted) {
    return Promise.reject(new Error("operation cancelled"));
  }

  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      signal.removeEventListener("abort", onAbort);
      resolve();
    }, ms);

    const onAbort = () => {
      clearTimeout(timer);
      reject(new Error("operation cancelled"));
    };
    signal.addEventListener("abort", onAbort, { once: true });
  });
}

export function runExec(
  command: string,
  args: string[],
  cwd: string,
  signal: AbortSignal,
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
