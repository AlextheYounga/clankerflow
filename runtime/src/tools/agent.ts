import { readFile } from "node:fs/promises";
import path from "node:path";

import { createOpencodeClient } from "@opencode-ai/sdk";

import type { RuntimeEnv } from "../protocol.ts";

const DEFAULT_OPENCODE_URL = "http://127.0.0.1:4096";

type JsonObject = Record<string, unknown>;

interface SessionClient {
  create(input?: JsonObject): Promise<unknown>;
  prompt(input: JsonObject): Promise<unknown>;
  messages(input: JsonObject): Promise<unknown>;
  abort(input: JsonObject): Promise<unknown>;
}

interface EventClient {
  subscribe(input?: JsonObject): Promise<unknown>;
}

interface OpenCodeClient {
  session: SessionClient;
  event: EventClient;
}

interface RuntimeSettings {
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

export function createAgent(options: AgentOptions): AgentContext {
  const runtime = createRuntime(options);

  return {
    async run(
      input: Record<string, unknown>
    ): Promise<Record<string, unknown>> {
      try {
        const client = await runtime.client();
        const prompt = requirePrompt(input.prompt);
        const session = await abortable(options.signal, () =>
          client.session.create(createSessionPayload(input))
        );
        const sessionId =
          getString(session, "id") ?? getString(session, "sessionID");
        if (sessionId === null) {
          throw new Error("OpenCode session.create did not return an id");
        }

        options.emitEvent("agent_session_started", {
          run_id: options.runId,
          session_id: sessionId,
        });

        const request = buildPromptRequest(input, sessionId, prompt, options.yolo);
        const message = await abortable(options.signal, () =>
          client.session.prompt(request)
        );
        const output =
          promptOutput(message) ??
          (await latestAssistantText(client, sessionId, options.signal)) ??
          "";

        return {
          ok: true,
          output,
          session_id: sessionId,
          message_id:
            getString(message, "id") ?? getString(message, "messageID"),
        };
      } catch (error) {
        return {
          ok: false,
          error: errorMessage(error),
        };
      }
    },

    async events(sessionId: string): Promise<Record<string, unknown>> {
      const client = await runtime.client();
      const rawStream = await abortable(options.signal, () =>
        client.event.subscribe()
      );
      return {
        session_id: sessionId,
        stream: filterEventsBySession(rawStream, sessionId),
      };
    },

    async messages(sessionId: string): Promise<Record<string, unknown>> {
      const client = await runtime.client();
      const messages = await abortable(options.signal, () =>
        client.session.messages({ path: { id: sessionId } })
      );
      return { session_id: sessionId, messages };
    },

    async cancel(sessionId: string): Promise<Record<string, unknown>> {
      const client = await runtime.client();
      const result = await abortable(options.signal, () =>
        client.session.abort({ path: { id: sessionId } })
      );
      return { session_id: sessionId, result };
    },
  };
}

export function normalizeServerUrl(
  url: string,
  runtimeEnv: RuntimeEnv
): string {
  if (runtimeEnv !== "container") {
    return url;
  }

  try {
    const parsed = new URL(url);
    if (parsed.hostname === "127.0.0.1" || parsed.hostname === "localhost") {
      parsed.hostname = "host.docker.internal";
    }
    return parsed.toString().replace(/\/$/, "");
  } catch {
    return url;
  }
}

function createRuntime(options: AgentOptions): {
  client(): Promise<OpenCodeClient>;
} {
  let instance: Promise<OpenCodeClient> | null = null;

  return {
    async client(): Promise<OpenCodeClient> {
      if (instance !== null) {
        return instance;
      }

      instance = (async () => {
        const loadSettings = options.loadSettings ?? loadRuntimeSettings;
        const settings = await loadSettings(options.workspaceRoot);
        const configured =
          typeof settings.opencode?.server_url === "string"
            ? settings.opencode.server_url
            : DEFAULT_OPENCODE_URL;
        const baseURL = normalizeServerUrl(configured, options.runtimeEnv);
        const createClient = options.createClient ?? createSdkClient;
        return createClient(baseURL);
      })();

      return instance;
    },
  };
}

async function loadRuntimeSettings(
  workspaceRoot: string
): Promise<RuntimeSettings> {
  const settingsPath = path.join(workspaceRoot, ".agents", "settings.json");
  const raw = await readFile(settingsPath, "utf8");
  return JSON.parse(raw) as RuntimeSettings;
}

function createSdkClient(baseURL: string): OpenCodeClient {
  return createOpencodeClient({
    baseUrl: baseURL,
  }) as unknown as OpenCodeClient;
}

function requirePrompt(prompt: unknown): string {
  if (typeof prompt === "string" && prompt.trim().length > 0) {
    return prompt;
  }
  throw new Error("agent.run requires a non-empty prompt");
}

function buildPromptRequest(
  input: Record<string, unknown>,
  sessionId: string,
  prompt: string,
  isYolo: boolean
): JsonObject {
  const request: JsonObject = {
    path: { id: sessionId },
    body: {
      parts: [{ type: "text", text: prompt }],
    },
  };
  const body = request.body as JsonObject;

  if (typeof input.system === "string") {
    body.system = input.system;
  }

  if (isRecord(input.tools)) {
    body.tools = input.tools;
  }

  if (typeof input.mode === "string") {
    body.mode = input.mode;
  }

  if (isRecord(input.model)) {
    if (typeof input.model.providerID === "string") {
      body.providerID = input.model.providerID;
    }
    if (typeof input.model.modelID === "string") {
      body.modelID = input.model.modelID;
    }
  }

  if (typeof input.provider_id === "string") {
    body.providerID = input.provider_id;
  }
  if (typeof input.model_id === "string") {
    body.modelID = input.model_id;
  }
  if (typeof input.providerID === "string") {
    body.providerID = input.providerID;
  }
  if (typeof input.modelID === "string") {
    body.modelID = input.modelID;
  }

  if (typeof input.title === "string" && input.title.trim().length > 0) {
    request.create = { title: input.title };
  }

  if (isYolo && typeof body.mode !== "string") {
    body.mode = "build";
  }

  return request;
}

function createSessionPayload(input: Record<string, unknown>): JsonObject {
  const title = typeof input.title === "string" ? input.title.trim() : "";
  if (title.length === 0) {
    return {};
  }

  return { body: { title } };
}

async function latestAssistantText(
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

  const responseEntries = response.filter(isRecord);
  const assistant = responseEntries
    .slice()
    .reverse()
    .find((entry) => getMessageRole(entry) === "assistant");

  return messageText(assistant);
}

function promptOutput(promptResponse: unknown): string | null {
  if (!isRecord(promptResponse)) {
    return null;
  }

  const info = promptResponse.info;
  if (isRecord(info)) {
    const infoContent = getString(info, "content") ?? getString(info, "text");
    if (infoContent !== null && infoContent.length > 0) {
      return infoContent;
    }
  }

  return messageText(promptResponse);
}

function messageText(message: unknown): string | null {
  if (!isRecord(message)) {
    return null;
  }

  const direct = getString(message, "content") ?? getString(message, "text");
  if (direct !== null && direct.length > 0) {
    return direct;
  }

  const parts = message.parts;
  if (!Array.isArray(parts)) {
    return null;
  }

  const chunks = parts
    .map((part) => {
      if (!isRecord(part)) {
        return null;
      }
      return getString(part, "text");
    })
    .filter((chunk): chunk is string => typeof chunk === "string");

  if (chunks.length === 0) {
    return null;
  }

  return chunks.join("\n");
}

function getString(record: unknown, key: string): string | null {
  if (!isRecord(record)) {
    return null;
  }
  const value = record[key];
  return typeof value === "string" ? value : null;
}

function getMessageRole(entry: Record<string, unknown>): string | null {
  const direct = getString(entry, "role");
  if (direct !== null) {
    return direct;
  }

  if (isRecord(entry.info)) {
    return getString(entry.info, "role");
  }

  return null;
}

function filterEventsBySession(stream: unknown, sessionId: string): unknown {
  if (Array.isArray(stream)) {
    return stream.filter((event) => eventSessionId(event) === sessionId);
  }

  if (isAsyncIterable(stream)) {
    return filterEventStream(stream, sessionId);
  }

  return stream;
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
  if (!isRecord(event)) {
    return null;
  }

  if (isRecord(event.properties)) {
    return getString(event.properties, "sessionID");
  }

  return null;
}

function isAsyncIterable(value: unknown): value is AsyncIterable<unknown> {
  return (
    typeof value === "object" &&
    value !== null &&
    Symbol.asyncIterator in value
  );
}

async function abortable<T>(
  signal: AbortSignal,
  operation: () => Promise<T>
): Promise<T> {
  if (signal.aborted) {
    throw new Error("operation cancelled");
  }

  return new Promise<T>((resolve, reject) => {
    const onAbort = () => {
      reject(new Error("operation cancelled"));
    };

    signal.addEventListener("abort", onAbort, { once: true });

    operation()
      .then(resolve)
      .catch(reject)
      .finally(() => {
        signal.removeEventListener("abort", onAbort);
      });
  });
}


function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
