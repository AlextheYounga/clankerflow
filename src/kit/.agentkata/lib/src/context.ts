import type { RuntimeEnv } from "./protocol.ts";
import {
  runExec,
  resolveExecSpec,
  sleepWithSignal,
  createLogContext,
  type EventEmitter,
} from "./utils.ts";

export { resolveExecSpec };

type CapabilityInvoker = (
  name: string,
  payload: Record<string, unknown>
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

const createAgentContext = (options: RunnerContextOptions) => ({
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
});

export function createContext(options: RunnerContextOptions) {
  return {
    yolo: options.yolo,
    ticket: options.ticket ?? null,
    agent: createAgentContext(options),
    exec: (command: string, args: string[] = []) => {
      const spec = resolveExecSpec(
        options.runtimeEnv,
        command,
        args,
        options.workspaceRoot
      );
      return runExec(spec.bin, spec.args, spec.cwd, options.signal);
    },
    log: createLogContext(options.emitEvent),
    sleep: (ms: number) => sleepWithSignal(ms, options.signal),
    signal: options.signal,
  };
}
