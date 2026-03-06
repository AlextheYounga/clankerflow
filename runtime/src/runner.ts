import readline from "node:readline";
import { createContext } from "./context.ts";
import { loadWorkflowModule } from "./loader.ts";
import {
  parseIpcMessage,
  type CancelRunPayload,
  type CapabilityErrorPayload,
  type CapabilityResponsePayload,
  type IpcMessage,
  type StartRunPayload,
} from "./protocol.ts";

type ActiveRun = {
  runId: string;
  controller: AbortController;
};

type PendingCapability = {
  resolve: (result: Record<string, unknown>) => void;
  reject: (error: Error) => void;
};

class Runner {
  private readonly activeRuns = new Map<string, ActiveRun>();
  private readonly pendingCapabilities = new Map<string, PendingCapability>();

  start(): void {
    const rl = readline.createInterface({
      input: process.stdin,
      crlfDelay: Infinity,
    });

    rl.on("line", async (line) => {
      if (!line.trim()) return;

      let message: IpcMessage;
      try {
        message = parseIpcMessage(line);
      } catch (error) {
        this.emit("error", "ipc_error", { message: errorMessage(error) });
        return;
      }

      try {
        await this.handleMessage(message);
      } catch (error) {
        this.emit("error", "runner_error", {
          message: errorMessage(error),
          command: message.name,
        });
      }
    });

    rl.on("close", async () => {
      for (const run of this.activeRuns.values()) {
        run.controller.abort();
      }
      await this.waitForRunDrain();
      process.exit(0);
    });
  }

  private async handleMessage(message: IpcMessage): Promise<void> {
    if (message.kind === "response") {
      const pending = this.pendingCapabilities.get(message.id);
      if (pending) {
        this.pendingCapabilities.delete(message.id);
        const payload = message.payload as unknown as CapabilityResponsePayload;
        pending.resolve(payload.result ?? message.payload);
      }
      return;
    }

    if (message.kind === "error") {
      const pending = this.pendingCapabilities.get(message.id);
      if (pending) {
        this.pendingCapabilities.delete(message.id);
        const payload = message.payload as unknown as CapabilityErrorPayload;
        pending.reject(new Error(payload.error ?? "capability request failed"));
      }
      return;
    }

    if (message.kind !== "command") return;

    if (message.name === "start_run") {
      const payload = message.payload as unknown as StartRunPayload;
      void this.executeRun(payload);
      return;
    }

    if (message.name === "cancel_run") {
      const payload = message.payload as unknown as CancelRunPayload;
      this.activeRuns.get(payload.run_id)?.controller.abort();
      return;
    }

    if (message.name === "shutdown") {
      for (const run of this.activeRuns.values()) {
        run.controller.abort();
      }
      await this.waitForRunDrain();
      process.exit(0);
    }
  }

  private async executeRun(payload: StartRunPayload): Promise<void> {
    const controller = new AbortController();
    this.activeRuns.set(payload.run_id, { runId: payload.run_id, controller });

    this.emit("event", "run_started", {
      run_id: payload.run_id,
      workflow_id: "unknown",
      workflow_name: "unknown",
      started_at: new Date().toISOString(),
    });

    this.emit("event", "step_started", {
      run_id: payload.run_id,
      step_id: `${payload.run_id}:workflow`,
      name: "workflow",
      started_at: new Date().toISOString(),
    });

    try {
      const module = await loadWorkflowModule(payload.workflow_path);
      this.emit("event", "log", {
        run_id: payload.run_id,
        level: "info",
        target: "runner",
        message: `loaded workflow ${module.meta.id}`,
        timestamp: new Date().toISOString(),
      });

      const ctx = createContext({
        workspaceRoot: process.cwd(),
        runtimeEnv: payload.runtime_env,
        yolo: payload.yolo,
        signal: controller.signal,
        ticket: payload.workflow_input?.ticket,
        emitEvent: (name, eventPayload) => {
          this.emit("event", name, {
            run_id: payload.run_id,
            ...eventPayload,
            timestamp: new Date().toISOString(),
          });
        },
        invokeCapability: (capability, params) =>
          this.sendCapabilityRequest(
            payload.run_id,
            capability,
            params,
            controller.signal,
          ),
      });

      await module.run(ctx);

      this.emit("event", "step_finished", {
        run_id: payload.run_id,
        step_id: `${payload.run_id}:workflow`,
        status: "ok",
        duration_ms: 0,
        finished_at: new Date().toISOString(),
      });
      this.emit("event", "run_finished", {
        run_id: payload.run_id,
        status: "SUCCEEDED",
        finished_at: new Date().toISOString(),
      });
    } catch (error) {
      if (controller.signal.aborted) {
        this.emit("event", "step_finished", {
          run_id: payload.run_id,
          step_id: `${payload.run_id}:workflow`,
          status: "cancelled",
          duration_ms: 0,
          finished_at: new Date().toISOString(),
        });
        this.emit("event", "run_finished", {
          run_id: payload.run_id,
          status: "CANCELLED",
          finished_at: new Date().toISOString(),
        });
      } else {
        this.emit("event", "step_finished", {
          run_id: payload.run_id,
          step_id: `${payload.run_id}:workflow`,
          status: "failed",
          duration_ms: 0,
          finished_at: new Date().toISOString(),
        });
        this.emit("event", "run_failed", {
          run_id: payload.run_id,
          error_code: "WORKFLOW_ERROR",
          message: errorMessage(error),
          details: {},
          failed_at: new Date().toISOString(),
        });
      }
    } finally {
      this.activeRuns.delete(payload.run_id);
    }
  }

  private sendCapabilityRequest(
    runId: string,
    capability: string,
    params: Record<string, unknown>,
    signal: AbortSignal,
  ): Promise<Record<string, unknown>> {
    const requestId = `req_${Date.now()}_${Math.random().toString(16).slice(2)}`;

    return new Promise((resolve, reject) => {
      if (signal.aborted) {
        reject(new Error("operation cancelled"));
        return;
      }

      const onAbort = () => {
        this.pendingCapabilities.delete(requestId);
        reject(new Error("operation cancelled"));
      };
      signal.addEventListener("abort", onAbort, { once: true });

      this.pendingCapabilities.set(requestId, {
        resolve: (result) => {
          signal.removeEventListener("abort", onAbort);
          resolve(result);
        },
        reject: (error) => {
          signal.removeEventListener("abort", onAbort);
          reject(error);
        },
      });

      this.emit("request", "capability_request", {
        request_id: requestId,
        run_id: runId,
        capability,
        params,
      });
    });
  }

  private async waitForRunDrain(): Promise<void> {
    while (this.activeRuns.size > 0) {
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
  }

  private emit(
    kind: IpcMessage["kind"],
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
    process.stdout.write(`${JSON.stringify(message)}\n`);
  }
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

new Runner().start();
