export interface AgentContext {
  run(input: Record<string, unknown>): Promise<Record<string, unknown>>;
  events(sessionId: string): Promise<Record<string, unknown>>;
  messages(sessionId: string): Promise<Record<string, unknown>>;
  cancel(sessionId: string): Promise<Record<string, unknown>>;
}

export interface AgentOptions {
  yolo: boolean;
  runId: number;
  signal: AbortSignal;
  emitEvent(name: string, payload: Record<string, unknown>): void;
  invokeCapability(
    name: string,
    payload: Record<string, unknown>,
    signal?: AbortSignal
  ): Promise<Record<string, unknown>>;
}
