import type { RuntimeEnv } from "../../protocol.ts";

export type JsonObject = Record<string, unknown>;

export interface SessionClient {
  create(input?: JsonObject): Promise<unknown>;
  prompt(input: JsonObject): Promise<unknown>;
  messages(input: JsonObject): Promise<unknown>;
  abort(input: JsonObject): Promise<unknown>;
}

export interface EventClient {
  subscribe(input?: JsonObject): Promise<unknown>;
}

export interface OpenCodeClient {
  session: SessionClient;
  event: EventClient;
}

export interface RuntimeSettings {
  opencode?: {
    server_url?: unknown;
  };
}

export interface AgentContext {
  run(input: Record<string, unknown>): Promise<Record<string, unknown>>;
  events(sessionId: string): Promise<Record<string, unknown>>;
  messages(sessionId: string): Promise<Record<string, unknown>>;
  cancel(sessionId: string): Promise<Record<string, unknown>>;
}

export interface AgentOptions {
  yolo: boolean;
  runId: number;
  runtimeEnv: RuntimeEnv;
  workspaceRoot: string;
  signal: AbortSignal;
  emitEvent(name: string, payload: Record<string, unknown>): void;
  createClient?: (baseUrl: string) => OpenCodeClient;
  loadSettings?: (workspaceRoot: string) => Promise<RuntimeSettings>;
}
