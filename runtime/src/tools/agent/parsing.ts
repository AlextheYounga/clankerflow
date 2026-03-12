import { abortable } from "./abort.ts";
import type {
  OpencodeClient,
  SessionMessagesResponse,
  SessionPromptResponse,
} from "./types.ts";

export async function latestAssistantText(
  client: OpencodeClient,
  sessionId: string,
  signal: AbortSignal
): Promise<string | null> {
  const response = await abortable(signal, () =>
    client.session.messages<true>({ path: { id: sessionId } })
  );

  const messages = response.data;

  const assistant = messages
    .slice()
    .reverse()
    .find((entry) => entry.info.role === "assistant");

  return assistant === undefined ? null : messageText(assistant.parts);
}

export function promptOutput(promptResponse: SessionPromptResponse): string | null {
  return messageText(promptResponse.parts);
}

export function filterEventsBySession(
  stream: unknown,
  sessionId: string
): unknown {
  if (Array.isArray(stream)) {
    return stream.filter((event) => eventSessionId(event) === sessionId);
  }

  if (isAsyncIterable(stream)) {
    return filterEventStream(stream, sessionId);
  }

  return stream;
}

function messageText(parts: SessionMessagesResponse[number]["parts"]): string | null {
  const chunks = parts
    .map((part) => (hasText(part) ? readString(part.text) : null))
    .filter((chunk): chunk is string => chunk !== null);

  if (chunks.length === 0) {
    return null;
  }

  return chunks.join("\n");
}

function hasText(value: unknown): value is { text: unknown } {
  return isRecord(value) && "text" in value;
}

async function* filterEventStream(
  stream: AsyncIterable<unknown>,
  sessionId: string
): AsyncIterable<unknown> {
  for await (const event of stream) {
    if (eventSessionId(event) === sessionId) {
      yield event;
    }
  }
}

function eventSessionId(event: unknown): string | null {
  if (!isRecord(event) || !isRecord(event.properties)) {
    return null;
  }

  return readString(event.properties.sessionID);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
  return (
    typeof value === "object" && value !== null && Symbol.asyncIterator in value
  );
}
