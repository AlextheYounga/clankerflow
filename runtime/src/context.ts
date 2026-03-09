import path from "node:path";
import type { RuntimeEnv } from "./protocol.ts";
import { runExec, sleepWithSignal } from "./helpers.ts";

type EventEmitter = (name: string, payload: Record<string, unknown>) => void;
type CapabilityInvoker = (
  name: string,
  payload: Record<string, unknown>,
) => Promise<Record<string, unknown>>;

export type RunnerContextOptions = {
  workspaceRoot: string;
  runtimeEnv: RuntimeEnv;
  yolo: boolean;
  signal: AbortSignal;
  emitEvent: EventEmitter;
  invokeCapability: CapabilityInvoker;
  ticket?: any;
};

export function resolveExecSpec(
  runtimeEnv: RuntimeEnv,
  command: string,
  args: string[],
  workspaceRoot: string,
): { bin: string; args: string[]; cwd: string } {
  if (runtimeEnv === "host") {
    return { bin: command, args, cwd: workspaceRoot };
  }

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
      "-T",
      "agent",
      command,
      ...args,
    ],
    cwd: workspaceRoot,
  };
}

export function createContext(options: RunnerContextOptions) {
  return {
    yolo: options.yolo,
    ticket: options.ticket || null,
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
    log: {
      debug: (message: string) =>
        options.emitEvent("log", {
          level: "debug",
          target: "workflow",
          message,
        }),
      info: (message: string) =>
        options.emitEvent("log", {
          level: "info",
          target: "workflow",
          message,
        }),
      warn: (message: string) =>
        options.emitEvent("log", {
          level: "warn",
          target: "workflow",
          message,
        }),
      error: (message: string) =>
        options.emitEvent("log", {
          level: "error",
          target: "workflow",
          message,
        }),
    },
    sleep: (ms: number) => sleepWithSignal(ms, options.signal),
    signal: options.signal,
  };
}
