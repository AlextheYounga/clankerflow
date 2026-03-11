import { IpcTransport, IpcRouter } from "./ipc.ts";
import { createContext } from "./context.ts";
import { createAgent } from "./tools/agent.ts";
import { createFsContext } from "./tools/fs.ts";
import { createGitContext } from "./tools/git.ts";
import { createTicketContext } from "./tools/tickets.ts";
import { loadWorkflowModule } from "./loader.ts";
import { createExec, createLogContext, sleepWithSignal } from "./utils.ts";
import type { StartRunPayload, CancelRunPayload } from "./protocol.ts";

interface ActiveRun {
  runId: number;
  controller: AbortController;
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

class Runner {
  private readonly activeRuns = new Map<number, ActiveRun>();
  private ipc: IpcRouter | null = null;

  start(): void {
    const transport = new IpcTransport();
    transport.start();
    transport.onDisconnect(() => this.shutdown());

    this.ipc = new IpcRouter(transport);
    this.ipc.start();

    this.ipc.onCommand("start_run", (payload) => {
      const startPayload = payload as unknown as StartRunPayload;
      // Command handlers stay non-blocking so one long workflow cannot stall
      // cancellation/shutdown commands for other active runs.
      void this.executeRun(startPayload);
    });

    this.ipc.onCommand("cancel_run", (payload) => {
      const cancelPayload = payload as unknown as CancelRunPayload;
      this.activeRuns.get(cancelPayload.run_id)?.controller.abort();
    });

    this.ipc.onCommand("shutdown", () => this.shutdown());
  }

  private emit(name: string, payload: Record<string, unknown>): void {
    this.ipc?.emit(name, payload);
  }

  private emitStep(runId: number, status: string): void {
    this.emit("step_finished", {
      run_id: runId,
      step_id: `${runId}:workflow`,
      status,
      duration_ms: 0,
      finished_at: new Date().toISOString(),
    });
  }

  private emitRunFinished(runId: number, status: string): void {
    this.emit("run_finished", {
      run_id: runId,
      status,
      finished_at: new Date().toISOString(),
    });
  }

  private async executeRun(payload: StartRunPayload): Promise<void> {
    const controller = new AbortController();
    this.activeRuns.set(payload.run_id, { runId: payload.run_id, controller });

    // Emit lifecycle events in deterministic order; Rust persists these as the
    // source of truth for run state transitions and UI timelines.
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
      await this.runWorkflow(payload, controller);
      this.emitStep(payload.run_id, "ok");
      this.emitRunFinished(payload.run_id, "SUCCEEDED");
    } catch (error) {
      this.emitRunError(payload.run_id, error, controller.signal.aborted);
    } finally {
      this.activeRuns.delete(payload.run_id);
    }
  }

  private async runWorkflow(
    payload: StartRunPayload,
    controller: AbortController
  ): Promise<void> {
    const module = await loadWorkflowModule(payload.workflow_path);
    this.emit("log", {
      run_id: payload.run_id,
      level: "info",
      target: "runner",
      message: `loaded workflow ${module.meta.id}`,
      timestamp: new Date().toISOString(),
    });

    const workspaceRoot = process.cwd();
    const signal = controller.signal;
    const emitEvent = (name: string, eventPayload: Record<string, unknown>) => {
      this.emit(name, {
        run_id: payload.run_id,
        ...eventPayload,
        timestamp: new Date().toISOString(),
      });
    };

    const context = createContext({
      workspaceRoot,
      runtimeEnv: payload.runtime_env,
      yolo: payload.yolo,
      signal,
      ticket: payload.workflow_input.ticket,
    });

    const agent = createAgent({
      yolo: payload.yolo,
      runId: payload.run_id,
      runtimeEnv: payload.runtime_env,
      workspaceRoot,
      signal,
      emitEvent,
    });

    const exec = createExec({
      runtimeEnv: payload.runtime_env,
      workspaceRoot,
      signal,
    });

    const log = createLogContext(emitEvent);
    const sleep = (ms: number) => sleepWithSignal(ms, signal);
    const fs = createFsContext(workspaceRoot);
    const git = createGitContext(workspaceRoot);
    const tickets = createTicketContext(workspaceRoot);

    await module.run(context, { agent, exec, log, sleep, fs, git, tickets });
  }

  private emitRunError(
    runId: number,
    error: unknown,
    isAborted: boolean
  ): void {
    if (isAborted) {
      this.emitStep(runId, "cancelled");
      this.emitRunFinished(runId, "CANCELLED");
    } else {
      this.emitStep(runId, "failed");
      this.emit("run_failed", {
        run_id: runId,
        error_code: "WORKFLOW_ERROR",
        message: errorMessage(error),
        details: {},
        failed_at: new Date().toISOString(),
      });
    }
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

new Runner().start();
