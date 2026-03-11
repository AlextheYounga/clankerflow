type CapabilityInvoker = (
  name: string,
  payload: Record<string, unknown>
) => Promise<Record<string, unknown>>;

export interface AgentContext {
  run(input: Record<string, unknown>): Promise<Record<string, unknown>>;
  events(sessionId: string): Promise<Record<string, unknown>>;
  messages(sessionId: string): Promise<Record<string, unknown>>;
  cancel(sessionId: string): Promise<Record<string, unknown>>;
}

export interface AgentOptions {
  yolo: boolean;
  invokeCapability: CapabilityInvoker;
}

export function createAgent(options: AgentOptions): AgentContext {
  return {
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
  };
}
