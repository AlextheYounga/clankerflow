import { IpcTransport, IpcRouter } from "./ipc.ts";
import { createContext } from "./context.ts";
import { loadWorkflowModule } from "./loader.ts";
import type {
  StartRunPayload,
  CancelRunPayload,
} from "./protocol.ts";

type ActiveRun = {
  runId: number;
  controller: AbortController;
};

class Runner {
  private readonly activeRuns = new Map<number, ActiveRun>();
  private ipc: IpcRouter | null = null;

  start(): void {
    const transport = new IpcTransport();
    transport.start();
    transport.onDisconnect(() => this.shutdown());

    this.ipc = new IpcRouter(transport);
    this.ipc.start();

    this.ipc.onCommand("start_run", async (payload) => {
      const startPayload = payload as unknown as StartRunPayload;
      void this.executeRun(startPayload);
    });

    this.ipc.onCommand("cancel_run", (payload) => {
      const cancelPayload = payload as unknown as CancelRunPayload;
      this.activeRuns.get(cancelPayload.run_id)?.controller.abort();
    });

    this.ipc.onCommand("shutdown", async () => this.shutdown());

  }

  private async executeRun(payload: StartRunPayload): Promise<void> {
    const controller = new AbortController();
    this.activeRuns.set(payload.run_id, { runId: payload.run_id, controller });

    this.emit("run_started", {
      run_id: payload.run_id,
      workflow_id: "unknown",
      workflow_name: "unknown",
      started_at: new Date().toISOString(),
    });

    this.emit("step_started", {
      run_id: payload.run_id,
      step_id: `${payload.run_id}:workflow`,
      name: "workflow",
      started_at: new Date().toISOString(),
    });

    try {
      const module = await loadWorkflowModule(payload.workflow_path);
      this.emit("log", {
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
          this.emit(name, {
            run_id: payload.run_id,
            ...eventPayload,
            timestamp: new Date().toISOString(),
          });
        },
        invokeCapability: (capability, params) =>
          this.ipc!.request(
            "capability_request",
            { run_id: payload.run_id, capability, params },
            controller.signal,
          ),
      });

      await module.run(ctx);

      this.emit("step_finished", {
        run_id: payload.run_id,
        step_id: `${payload.run_id}:workflow`,
        status: "ok",
        duration_ms: 0,
        finished_at: new Date().toISOString(),
      });
      this.emit("run_finished", {
        run_id: payload.run_id,
        status: "SUCCEEDED",
        finished_at: new Date().toISOString(),
      });
    } catch (error) {
      if (controller.signal.aborted) {
        this.emit("step_finished", {
          run_id: payload.run_id,
          step_id: `${payload.run_id}:workflow`,
          status: "cancelled",
          duration_ms: 0,
          finished_at: new Date().toISOString(),
        });
        this.emit("run_finished", {
          run_id: payload.run_id,
          status: "CANCELLED",
          finished_at: new Date().toISOString(),
        });
      } else {
        this.emit("step_finished", {
          run_id: payload.run_id,
          step_id: `${payload.run_id}:workflow`,
          status: "failed",
          duration_ms: 0,
          finished_at: new Date().toISOString(),
        });
        this.emit("run_failed", {
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

  private emit(name: string, payload: Record<string, unknown>): void {
    this.ipc?.emit(name, payload);
  }

  private async waitForRunDrain(): Promise<void> {
    while (this.activeRuns.size > 0) {
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
  }

  private async shutdown(): Promise<void> {
    for (const run of this.activeRuns.values()) {
      run.controller.abort();
    }
    await this.waitForRunDrain();
    process.exit(0);
  }
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

new Runner().start();
