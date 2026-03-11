import { abortable } from "./abort.ts";
import type { OpenCodeClient } from "./types.ts";

export async function latestAssistantText(
  client: OpenCodeClient,
  sessionId: string,
  signal: AbortSignal
): Promise<string | null> {
  const response = await abortable(signal, () =>
    client.session.messages({ path: { id: sessionId } })
  );

  if (!Array.isArray(response)) {
    return null;
  }

  const assistant = response
    .filter(isRecord)
    .slice()
    .reverse()
    .find((entry) => messageRole(entry) === "assistant");

  return messageText(assistant);
}

export function promptOutput(promptResponse: unknown): string | null {
  if (!isRecord(promptResponse)) {
    return null;
  }

  if (isRecord(promptResponse.info)) {
    const infoText = readMessageText(promptResponse.info);
    if (infoText !== null) {
      return infoText;
    }
  }

  return messageText(promptResponse);
}

export function filterEventsBySession(stream: unknown, sessionId: string): unknown {
  if (Array.isArray(stream)) {
    return stream.filter((event) => eventSessionId(event) === sessionId);
  }

  if (isAsyncIterable(stream)) {
    return filterEventStream(stream, sessionId);
  }

  return stream;
}

function messageText(message: unknown): string | null {
  if (!isRecord(message)) {
    return null;
  }

  const directText = readMessageText(message);
  if (directText !== null) {
    return directText;
  }

  const parts = message.parts;
  if (!Array.isArray(parts)) {
    return null;
  }

  const chunks = parts
    .map((part) => (isRecord(part) ? readString(part.text) : null))
    .filter((chunk): chunk is string => chunk !== null);

  if (chunks.length === 0) {
    return null;
  }

  return chunks.join("\n");
}

function readMessageText(record: Record<string, unknown>): string | null {
  const content = readString(record.content);
  if (content !== null && content.length > 0) {
    return content;
  }

  const text = readString(record.text);
  if (text !== null && text.length > 0) {
    return text;
  }

  return null;
}

function messageRole(record: Record<string, unknown>): string | null {
  const direct = readString(record.role);
  if (direct !== null) {
    return direct;
  }

  if (isRecord(record.info)) {
    return readString(record.info.role);
  }

  return null;
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
    typeof value === "object" &&
    value !== null &&
    Symbol.asyncIterator in value
  );
}
