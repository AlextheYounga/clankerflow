export type RuntimeEnv = "host" | "container";

export type IpcMessageKind =
  | "command"
  | "event"
  | "request"
  | "response"
  | "error";

export interface IpcMessage {
  v: "v1";
  id: string;
  kind: IpcMessageKind;
  name: string;
  payload: Record<string, unknown>;
}

export interface StartRunPayload {
  run_id: number;
  workflow_path: string;
  runtime_env: RuntimeEnv;
  yolo: boolean;
  workflow_input: Record<string, unknown>;
}

export interface CancelRunPayload {
  run_id: number;
  reason: string;
}

export interface CapabilityRequestPayload {
  request_id: string;
  run_id: number;
  capability: string;
  params: Record<string, unknown>;
}

export interface CapabilityResponsePayload {
  result: Record<string, unknown>;
}

export interface CapabilityErrorPayload {
  error: string;
}

function isVersionedObject(value: unknown): value is Record<string, unknown> {
  return (
    typeof value === "object" &&
    value !== null &&
    "v" in value &&
    (value as Record<string, unknown>).v === "v1"
  );
}

function hasRequiredFields(msg: Record<string, unknown>): boolean {
  return (
    typeof msg.id === "string" &&
    typeof msg.kind === "string" &&
    typeof msg.name === "string" &&
    typeof msg.payload === "object" &&
    msg.payload !== null
  );
}

export function parseIpcMessage(input: unknown): IpcMessage {
  const parsed: unknown =
    typeof input === "string" ? (JSON.parse(input) as unknown) : input;

  if (!isVersionedObject(parsed)) {
    throw new Error("unsupported protocol version");
  }

  if (!hasRequiredFields(parsed)) {
    throw new Error("invalid IPC message");
  }

  return parsed as unknown as IpcMessage;
}
