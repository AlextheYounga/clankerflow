import type { RuntimeEnv } from "./protocol.ts";
import type { Ticket } from "./tools/tickets/parser.ts";
import { toContextTicket } from "./tools/tickets.ts";
import { resolveExecSpec } from "./utils.ts";

export { resolveExecSpec };

export interface ContextOptions {
  workspaceRoot: string;
  runtimeEnv: RuntimeEnv;
  yolo: boolean;
  signal: AbortSignal;
  ticket?: unknown;
}

export interface WorkflowContext {
  workspaceRoot: string;
  runtimeEnv: RuntimeEnv;
  yolo: boolean;
  ticket: Ticket | null;
  signal: AbortSignal;
}

export function createContext(options: ContextOptions): WorkflowContext {
  return {
    workspaceRoot: options.workspaceRoot,
    runtimeEnv: options.runtimeEnv,
    yolo: options.yolo,
    ticket: toContextTicket(options.ticket),
    signal: options.signal,
  };
}
