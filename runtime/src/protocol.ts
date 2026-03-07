export type RuntimeEnv = "host" | "container";

export type IpcMessageKind =
  | "command"
  | "event"
  | "request"
  | "response"
  | "error";

export type IpcMessage = {
  v: "v1";
  id: string;
  kind: IpcMessageKind;
  name: string;
  payload: Record<string, unknown>;
};

export type StartRunPayload = {
  run_id: string;
  workflow_path: string;
  runtime_env: RuntimeEnv;
  yolo: boolean;
  workflow_input: Record<string, unknown>;
};

export type CancelRunPayload = {
  run_id: string;
  reason: string;
};

export type CapabilityRequestPayload = {
  request_id: string;
  run_id: string;
  capability: string;
  params: Record<string, unknown>;
};

export type CapabilityResponsePayload = {
  result: Record<string, unknown>;
};

export type CapabilityErrorPayload = {
  error: string;
};

export function parseIpcMessage(input: unknown): IpcMessage {
  const parsed =
    typeof input === "string"
      ? (JSON.parse(input) as IpcMessage)
      : (input as IpcMessage);
  if (parsed.v !== "v1") {
    throw new Error("unsupported protocol version");
  }
  if (!parsed.id || !parsed.kind || !parsed.name || !parsed.payload) {
    throw new Error("invalid IPC message");
  }
  return parsed;
}
