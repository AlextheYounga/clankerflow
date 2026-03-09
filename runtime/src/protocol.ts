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

export function parseIpcMessage(input: unknown): IpcMessage {
  const parsed: unknown =
    typeof input === "string" ? (JSON.parse(input) as unknown) : input;

  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("v" in parsed) ||
    (parsed as Record<string, unknown>).v !== "v1"
  ) {
    throw new Error("unsupported protocol version");
  }

  const msg = parsed as Record<string, unknown>;
  if (
    typeof msg.id !== "string" ||
    typeof msg.kind !== "string" ||
    typeof msg.name !== "string" ||
    typeof msg.payload !== "object" ||
    msg.payload === null
  ) {
    throw new Error("invalid IPC message");
  }

  return parsed as IpcMessage;
}
