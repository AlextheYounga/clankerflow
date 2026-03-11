import { readFile } from "node:fs/promises";
import path from "node:path";

import { createOpencodeClient } from "@opencode-ai/sdk";

import type { RuntimeEnv } from "../protocol.ts";

const DEFAULT_OPENCODE_URL = "http://127.0.0.1:4096";

type JsonObject = Record<string, unknown>;

interface SessionClient {
  create(input?: JsonObject): Promise<unknown>;
  chat(sessionId: string, input: JsonObject): Promise<unknown>;
  messages(sessionId: string): Promise<unknown>;
  abort(sessionId: string): Promise<unknown>;
}

interface EventClient {
  list(): Promise<unknown>;
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
        const session = await client.session.create();
        const sessionId =
          getString(session, "id") ?? getString(session, "sessionID");
        if (sessionId === null) {
          throw new Error("OpenCode session.create did not return an id");
        }

        options.emitEvent("agent_session_started", {
          run_id: options.runId,
          session_id: sessionId,
        });

        const body = buildChatInput(input, prompt, options.yolo);
        const message = await client.session.chat(sessionId, body);
        const output =
          messageText(message) ??
          (await latestAssistantText(client, sessionId)) ??
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
      const stream = await client.event.list();
      return { session_id: sessionId, stream };
    },

    async messages(sessionId: string): Promise<Record<string, unknown>> {
      const client = await runtime.client();
      const messages = await client.session.messages(sessionId);
      return { session_id: sessionId, messages };
    },

    async cancel(sessionId: string): Promise<Record<string, unknown>> {
      const client = await runtime.client();
      const result = await client.session.abort(sessionId);
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

function buildChatInput(
  input: Record<string, unknown>,
  prompt: string,
  yolo: boolean
): JsonObject {
  const body: JsonObject = {
    parts: [{ type: "text", text: prompt }],
  };

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

  if (yolo && typeof body.mode !== "string") {
    body.mode = "build";
  }

  return body;
}

async function latestAssistantText(
  client: OpenCodeClient,
  sessionId: string
): Promise<string | null> {
  const response = await client.session.messages(sessionId);
  if (!Array.isArray(response)) {
    return null;
  }

  const responseEntries = response.filter(isRecord);
  const assistant = responseEntries
    .slice()
    .reverse()
    .find((entry) => getString(entry, "role") === "assistant");

  return messageText(assistant);
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
