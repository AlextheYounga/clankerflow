import net from "net";

import {
  parseIpcMessage,
  type IpcMessage,
  type IpcMessageKind,
} from "./protocol.ts";

type CommandHandler = (
  payload: Record<string, unknown>
) => void | Promise<void>;

export class IpcTransport {
  private socket: net.Socket | null = null;
  private messageHandler: ((message: IpcMessage) => void) | null = null;
  private disconnectHandler: (() => void | Promise<void>) | null = null;

  start(): void {
    const rawPort = process.env.AGENTCTL_IPC_PORT;
    if (!rawPort) {
      process.stderr.write("fatal: AGENTCTL_IPC_PORT is not set\n");
      process.exit(1);
    }
    const port = Number.parseInt(rawPort, 10);
    const host =
      process.env.AGENTCTL_CONTAINER === "1"
        ? "host.docker.internal"
        : "127.0.0.1";

    this.socket = net.createConnection({ host, port });

    let buffer = "";
    this.socket.on("data", (chunk) => {
      buffer += chunk.toString();
      const lines = buffer.split("\n");
      buffer = lines.pop()!;
      for (const line of lines) {
        if (line.trim()) {
          try {
            const message = parseIpcMessage(JSON.parse(line));
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
        }
      }
    });

    this.socket.on("error", (error) => {
      process.stderr.write(`ipc connection error: ${errorMessage(error)}\n`);
    });

    this.socket.on("close", () => {
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
    if (this.socket === null || this.socket.destroyed) {
      return;
    }

    this.socket.write(JSON.stringify(message) + "\n", (error) => {
      if (error) {
        process.stderr.write(`ipc send error: ${errorMessage(error)}\n`);
      }
    });
  }
}

export class IpcRouter {
  private readonly transport: IpcTransport;
  private readonly commandHandlers = new Map<string, CommandHandler>();

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
    payload: Record<string, unknown>
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
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}
