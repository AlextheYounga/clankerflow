import type { AgentContext, AgentOptions } from "./agent/types.ts";

export type { AgentContext, AgentOptions };

export function createAgent(options: AgentOptions): AgentContext {
  return {
    run: createRunHandler(options),
    command: (input) =>
      options.invokeCapability("opencode_command", input, options.signal),
    events: (sessionId) =>
      options.invokeCapability(
        "opencode_events",
        { session_id: sessionId },
        options.signal
      ),
    messages: (sessionId) =>
      options.invokeCapability(
        "opencode_messages",
        { session_id: sessionId },
        options.signal
      ),
    cancel: (sessionId) =>
      options.invokeCapability(
        "opencode_cancel",
        { session_id: sessionId },
        options.signal
      ),
  };
}

function createRunHandler(options: AgentOptions): AgentContext["run"] {
  return async (input) => {
    try {
      const payload = await options.invokeCapability(
        "opencode_run",
        {
          yolo: options.yolo,
          ...input,
        },
        options.signal
      );

      const sessionId = readString(payload.session_id);
      if (sessionId !== null) {
        options.emitEvent("agent_session_started", {
          run_id: options.runId,
          session_id: sessionId,
        });
      }

      return {
        ok: true,
        output: readString(payload.output) ?? "",
        session_id: sessionId,
        message_id: readString(payload.message_id),
      };
    } catch (error) {
      return { ok: false, error: errorMessage(error) };
    }
  };
}

function readString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function errorMessage(error: unknown): string {
  if (!(error instanceof Error)) {
    return String(error);
  }

  if (error.cause instanceof Error) {
    return `${error.message}: ${error.cause.message}`;
  }

  return error.message;
}
