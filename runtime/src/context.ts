import path from "node:path";

import type { RuntimeEnv } from "./protocol.ts";
import { runExec, sleepWithSignal } from "./helpers.ts";

type EventEmitter = (name: string, payload: Record<string, unknown>) => void;
type CapabilityInvoker = (
  name: string,
  payload: Record<string, unknown>,
) => Promise<Record<string, unknown>>;

export interface RunnerContextOptions {
  workspaceRoot: string;
  runtimeEnv: RuntimeEnv;
  yolo: boolean;
  signal: AbortSignal;
  emitEvent: EventEmitter;
  invokeCapability: CapabilityInvoker;
  ticket?: unknown;
}

export function resolveExecSpec(
  runtimeEnv: RuntimeEnv,
  command: string,
  args: string[],
  workspaceRoot: string,
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
        ".agentctl",
        "docker",
        "agent.docker-compose.yaml",
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

function createLogContext(emit: EventEmitter) {
  const log = (level: string, message: string) =>
    emit("log", { level, target: "workflow", message });
  return {
    debug: (message: string) => log("debug", message),
    info: (message: string) => log("info", message),
    warn: (message: string) => log("warn", message),
    error: (message: string) => log("error", message),
  };
}

export function createContext(options: RunnerContextOptions) {
  return {
    yolo: options.yolo,
    ticket: options.ticket ?? null,
    agent: {
      run: (input: Record<string, unknown>) =>
        options.invokeCapability("session_run", {
          yolo: options.yolo,
          ...input,
        }),
      events: (sessionId: string) =>
        options.invokeCapability("session_events_subscribe", {
          session_id: sessionId,
        }),
      messages: (sessionId: string) =>
        options.invokeCapability("session_messages_list", {
          session_id: sessionId,
        }),
      cancel: (sessionId: string) =>
        options.invokeCapability("session_cancel", { session_id: sessionId }),
    },
    exec: (command: string, args: string[] = []) => {
      const spec = resolveExecSpec(
        options.runtimeEnv,
        command,
        args,
        options.workspaceRoot,
      );
      return runExec(spec.bin, spec.args, spec.cwd, options.signal);
    },
    log: createLogContext(options.emitEvent),
    sleep: (ms: number) => sleepWithSignal(ms, options.signal),
    signal: options.signal,
  };
}
