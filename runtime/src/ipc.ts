import {
  parseIpcMessage,
  type IpcMessage,
  type IpcMessageKind,
} from "./protocol.ts";

type CommandHandler = (
  payload: Record<string, unknown>,
) => void | Promise<void>;
interface PendingRequest {
  resolve: (result: Record<string, unknown>) => void;
  reject: (error: Error) => void;
}

export class IpcTransport {
  private messageHandler: ((message: IpcMessage) => void) | null = null;
  private disconnectHandler: (() => void | Promise<void>) | null = null;
  private readonly proc: NodeJS.Process;

  constructor(proc: NodeJS.Process = process) {
    this.proc = proc;
  }

  start(): void {
    this.proc.on("message", (raw: unknown) => {
      try {
        const message = parseIpcMessage(raw);
        this.messageHandler?.(message);
      } catch (error) {
        this.send({
          v: "v1",
          id: `err_${Date.now()}`,
          kind: "error",
          name: "ipc_parse_error",
          payload: { message: errorMessage(error) },
        });
      }
    });

    this.proc.on("disconnect", () => {
      void this.disconnectHandler?.();
    });
  }

  onMessage(handler: (message: IpcMessage) => void): void {
    this.messageHandler = handler;
  }

  onDisconnect(handler: () => void | Promise<void>): void {
    this.disconnectHandler = handler;
  }

  send(message: IpcMessage): void {
    if (typeof this.proc.send !== "function" || this.proc.connected !== true) {
      return;
    }

    this.proc.send(message, (error: Error | null) => {
      if (error) {
        process.stderr.write(`ipc send error: ${errorMessage(error)}\n`);
      }
    });
  }
}

export class IpcRouter {
  private readonly transport: IpcTransport;
  private readonly commandHandlers = new Map<string, CommandHandler>();
  private readonly pendingRequests = new Map<string, PendingRequest>();

  constructor(transport: IpcTransport) {
    this.transport = transport;
  }

  start(): void {
    this.transport.onMessage((message) => {
      void this.handleMessage(message);
    });
  }

  onCommand(name: string, handler: CommandHandler): void {
    this.commandHandlers.set(name, handler);
  }

  private async handleMessage(message: IpcMessage): Promise<void> {
    if (message.kind === "response") {
      const pending = this.pendingRequests.get(message.id);
      if (pending) {
        this.pendingRequests.delete(message.id);
        pending.resolve(message.payload);
      }
      return;
    }

    if (message.kind === "error") {
      const pending = this.pendingRequests.get(message.id);
      if (pending) {
        this.pendingRequests.delete(message.id);
        const payload = message.payload as { error?: string };
        pending.reject(new Error(payload.error ?? "request failed"));
      }
      return;
    }

    if (message.kind === "command") {
      const handler = this.commandHandlers.get(message.name);
      if (handler) {
        try {
          await handler(message.payload);
        } catch (error) {
          this.transport.send({
            v: "v1",
            id: message.id,
            kind: "error",
            name: "command_error",
            payload: { message: errorMessage(error), command: message.name },
          });
        }
      }
      return;
    }
  }

  send(
    kind: IpcMessageKind,
    name: string,
    payload: Record<string, unknown>,
  ): void {
    const message: IpcMessage = {
      v: "v1",
      id: `msg_${Date.now()}_${Math.random().toString(16).slice(2)}`,
      kind,
      name,
      payload,
    };
    this.transport.send(message);
  }

  emit(name: string, payload: Record<string, unknown>): void {
    this.send("event", name, payload);
  }

  request(
    name: string,
    payload: Record<string, unknown>,
    signal?: AbortSignal,
  ): Promise<Record<string, unknown>> {
    const requestId = `req_${Date.now()}_${Math.random().toString(16).slice(2)}`;

    return new Promise((resolve, reject) => {
      if (signal?.aborted === true) {
        reject(new Error("operation cancelled"));
        return;
      }

      const abortHandler = () => {
        this.pendingRequests.delete(requestId);
        reject(new Error("operation cancelled"));
      };

      if (signal) {
        signal.addEventListener("abort", abortHandler, { once: true });
      }

      this.pendingRequests.set(requestId, {
        resolve: (result) => {
          signal?.removeEventListener("abort", abortHandler);
          resolve(result);
        },
        reject: (error) => {
          signal?.removeEventListener("abort", abortHandler);
          reject(error);
        },
      });

      this.transport.send({
        v: "v1",
        id: requestId,
        kind: "request",
        name,
        payload: { ...payload, request_id: requestId },
      });
    });
  }
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}
