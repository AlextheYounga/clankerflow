import path from "node:path";

import type { AgentContext } from "./tools/agent.ts";
import type { WorkflowContext } from "./context.ts";
import type { FsContext } from "./tools/fs.ts";
import type { GitContext } from "./tools/git.ts";
import type { TicketContext } from "./tools/tickets.ts";
import type { ExecContext } from "./tools/exec.ts";
import type { LogContext } from "./tools/log.ts";

export interface WorkflowMeta {
  id: string;
  name: string;
  runtime: "host" | "container";
}

export interface WorkflowTools {
  agent: AgentContext;
  exec: ExecContext;
  log: LogContext;
  sleep(ms: number): Promise<void>;
  fs: FsContext;
  git: GitContext;
  tickets: TicketContext;
}

export type WorkflowRun = (
  context: WorkflowContext,
  tools: WorkflowTools
) => Promise<void>;

export interface WorkflowModule {
  meta: WorkflowMeta;
  run: WorkflowRun;
}

export async function loadWorkflowModule(
  workflowPath: string
): Promise<WorkflowModule> {
  const normalizedPath = path.resolve(workflowPath);
  const moduleUrl = pathToFileUrl(normalizedPath);
  const loaded = (await import(moduleUrl)) as {
    meta?: unknown;
    default?: unknown;
  };

  const meta = validateMeta(loaded.meta);
  const run = validateDefaultRun(loaded.default);

  return { meta, run };
}

function validateMeta(input: unknown): WorkflowMeta {
  if (input === null || input === undefined || typeof input !== "object") {
    throw new Error("workflow meta export is required");
  }
  const meta = input as Partial<WorkflowMeta>;
  if (typeof meta.id !== "string" || meta.id.length === 0) {
    throw new Error("workflow meta.id must be a non-empty string");
  }
  if (typeof meta.name !== "string" || meta.name.length === 0) {
    throw new Error("workflow meta.name must be a non-empty string");
  }
  if (meta.runtime !== "host" && meta.runtime !== "container") {
    throw new Error("workflow meta.runtime must be either host or container");
  }
  return meta as WorkflowMeta;
}

function validateDefaultRun(input: unknown): WorkflowRun {
  if (typeof input !== "function") {
    throw new Error("workflow default export must be an async function");
  }
  const constructorName = input.constructor.name;
  if (constructorName !== "AsyncFunction") {
    throw new Error("workflow default export must be an async function");
  }
  return input as WorkflowRun;
}

function pathToFileUrl(filePath: string): string {
  // Dynamic import expects file URLs; slash normalization keeps Windows paths
  // valid without relying on process-global URL helpers.
  const normalized = filePath.replace(/\\/g, "/");
  return `file://${normalized}`;
}
