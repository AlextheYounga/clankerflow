import type { OpencodeClient } from "@opencode-ai/sdk";

import type { RuntimeEnv } from "../../protocol.ts";
export type { OpencodeClient };
export type JsonObject = Record<string, unknown>;

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
  createClient?: (baseUrl: string) => OpencodeClient;
  loadSettings?: (workspaceRoot: string) => Promise<RuntimeSettings>;
}
