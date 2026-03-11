import type { JsonObject } from "./types.ts";

export function createSessionPayload(input: Record<string, unknown>): JsonObject {
  const title = readTrimmedString(input.title);
  if (title === null) {
    return {};
  }

  return { body: { title } };
}

export function buildPromptRequest(
  input: Record<string, unknown>,
  sessionId: string,
  prompt: string,
  isYolo: boolean
): JsonObject {
  const body: JsonObject = {
    parts: [{ type: "text", text: prompt }],
  };
  const request: JsonObject = {
    path: { id: sessionId },
    body,
  };

  applyString(body, "system", input.system);
  applyString(body, "mode", input.mode);
  applyRecord(body, "tools", input.tools);
  applyModel(body, input.model);
  applyModelIds(body, input);

  if (isYolo && typeof body.mode !== "string") {
    body.mode = "build";
  }

  return request;
}

function applyString(target: JsonObject, key: string, value: unknown): void {
  if (typeof value === "string") {
    target[key] = value;
  }
}

function applyRecord(target: JsonObject, key: string, value: unknown): void {
  if (isRecord(value)) {
    target[key] = value;
  }
}

function applyModel(target: JsonObject, value: unknown): void {
  if (!isRecord(value)) {
    return;
  }

  if (typeof value.providerID === "string" && typeof value.modelID === "string") {
    target.model = {
      providerID: value.providerID,
      modelID: value.modelID,
    };
  }
}

function applyModelIds(
  target: JsonObject,
  input: Record<string, unknown>
): void {
  const providerId =
    readString(input.providerID) ?? readString(input.provider_id);
  const modelId = readString(input.modelID) ?? readString(input.model_id);

  if (providerId !== null && modelId !== null) {
    target.model = {
      providerID: providerId,
      modelID: modelId,
    };
  }
}

function readTrimmedString(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }

  const trimmed = value.trim();
  if (trimmed.length === 0) {
    return null;
  }

  return trimmed;
}

function readString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
