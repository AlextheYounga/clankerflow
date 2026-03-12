import {
  filterEventsBySession,
  latestAssistantText,
  promptOutput,
} from "./agent/parsing.ts";
import { buildPromptRequest, createSessionPayload } from "./agent/payload.ts";
import { createRuntime } from "./agent/runtime.ts";
import { normalizeServerUrl } from "./agent/server-url.ts";
import type {
  AgentContext,
  AgentOptions,
  OpencodeClient,
  SessionPromptResponse,
} from "./agent/types.ts";
import { abortable } from "./agent/abort.ts";

export { normalizeServerUrl };
export type { AgentContext, AgentOptions };

export function createAgent(options: AgentOptions): AgentContext {
  const runtime = createRuntime(options);
  return {
    run: createRunHandler(options, () => runtime.client()),
    events: createEventsHandler(options, () => runtime.client()),
    messages: createMessagesHandler(options, () => runtime.client()),
    cancel: createCancelHandler(options, () => runtime.client()),
  };
}

function createRunHandler(
  options: AgentOptions,
  getClient: () => Promise<OpencodeClient>
): AgentContext["run"] {
  return async (input) => {
    try {
      const client = await getClient();
      const sessionId = await startSession(options, client, input);
      const message = await sendPrompt(options, client, sessionId, input);
      const output = await resolveOutput(options, client, sessionId, message);
      const messageId = readMessageId(message);

      return {
        ok: true,
        output,
        session_id: sessionId,
        message_id: messageId,
      };
    } catch (error) {
      return { ok: false, error: errorMessage(error) };
    }
  };
}

function createEventsHandler(
  options: AgentOptions,
  getClient: () => Promise<OpencodeClient>
): AgentContext["events"] {
  return async (sessionId) => {
    const client = await getClient();
    const rawStream = await abortable(options.signal, () =>
      client.event.subscribe()
    );
    return {
      session_id: sessionId,
      stream: filterEventsBySession(rawStream, sessionId),
    };
  };
}

function createMessagesHandler(
  options: AgentOptions,
  getClient: () => Promise<OpencodeClient>
): AgentContext["messages"] {
  return async (sessionId) => {
    const client = await getClient();
    const response = await abortable(options.signal, () =>
      client.session.messages<true>({ path: { id: sessionId } })
    );
    return { session_id: sessionId, messages: response.data };
  };
}

function createCancelHandler(
  options: AgentOptions,
  getClient: () => Promise<OpencodeClient>
): AgentContext["cancel"] {
  return async (sessionId) => {
    const client = await getClient();
    const response = await abortable(options.signal, () =>
      client.session.abort<true>({ path: { id: sessionId } })
    );
    return { session_id: sessionId, result: response.data };
  };
}

async function startSession(
  options: AgentOptions,
  client: OpencodeClient,
  input: Record<string, unknown>
): Promise<string> {
  const response = await abortable(options.signal, () =>
    client.session.create<true>(createSessionPayload(input))
  );
  const sessionId = response.data.id;
  options.emitEvent("agent_session_started", {
    run_id: options.runId,
    session_id: sessionId,
  });
  return sessionId;
}

async function sendPrompt(
  options: AgentOptions,
  client: OpencodeClient,
  sessionId: string,
  input: Record<string, unknown>
): Promise<SessionPromptResponse> {
  const prompt = requirePrompt(input.prompt);
  const request = buildPromptRequest(input, sessionId, prompt);
  const response = await abortable(options.signal, () =>
    client.session.prompt<true>(request)
  );
  return response.data;
}

async function resolveOutput(
  options: AgentOptions,
  client: OpencodeClient,
  sessionId: string,
  promptResponse: SessionPromptResponse
): Promise<string> {
  return (
    promptOutput(promptResponse) ??
    (await latestAssistantText(client, sessionId, options.signal)) ??
    ""
  );
}

function readMessageId(message: SessionPromptResponse): string {
  return message.info.id;
}

function requirePrompt(prompt: unknown): string {
  if (typeof prompt === "string" && prompt.trim().length > 0) {
    return prompt;
  }

  throw new Error("agent.run requires a non-empty prompt");
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
