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
    const messages = await abortable(options.signal, () =>
      client.session.messages({ path: { id: sessionId } })
    );
    return { session_id: sessionId, messages };
  };
}

function createCancelHandler(
  options: AgentOptions,
  getClient: () => Promise<OpencodeClient>
): AgentContext["cancel"] {
  return async (sessionId) => {
    const client = await getClient();
    const result = await abortable(options.signal, () =>
      client.session.abort({ path: { id: sessionId } })
    );
    return { session_id: sessionId, result };
  };
}

async function startSession(
  options: AgentOptions,
  client: OpencodeClient,
  input: Record<string, unknown>
): Promise<string> {
  const session = await abortable(options.signal, () =>
    client.session.create(createSessionPayload(input))
  );
  const sessionId = readSessionId(session);
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
): Promise<unknown> {
  const prompt = requirePrompt(input.prompt);
  const request = buildPromptRequest(input, sessionId, prompt);
  return abortable(options.signal, () => client.session.prompt(request));
}

async function resolveOutput(
  options: AgentOptions,
  client: OpencodeClient,
  sessionId: string,
  promptResponse: unknown
): Promise<string> {
  return (
    promptOutput(promptResponse) ??
    (await latestAssistantText(client, sessionId, options.signal)) ??
    ""
  );
}

function readSessionId(session: unknown): string {
  if (
    typeof session === "object" &&
    session !== null &&
    "id" in session &&
    typeof session.id === "string"
  ) {
    return session.id;
  }

  throw new Error("OpenCode session.create did not return an id");
}

function readMessageId(message: unknown): string | null {
  if (typeof message !== "object" || message === null) {
    return null;
  }

  if ("id" in message && typeof message.id === "string") {
    return message.id;
  }

  if (
    "info" in message &&
    typeof message.info === "object" &&
    message.info !== null &&
    "id" in message.info &&
    typeof message.info.id === "string"
  ) {
    return message.info.id;
  }

  return null;
}

function requirePrompt(prompt: unknown): string {
  if (typeof prompt === "string" && prompt.trim().length > 0) {
    return prompt;
  }

  throw new Error("agent.run requires a non-empty prompt");
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
