import path from "node:path";

export interface WorkflowMeta {
  id: string;
  name: string;
  runtime: "host" | "container";
}

export interface WorkflowModule {
  meta: WorkflowMeta;
  run: (ctx: unknown) => Promise<void>;
}

export async function loadWorkflowModule(
  workflowPath: string,
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

function validateDefaultRun(input: unknown): (ctx: unknown) => Promise<void> {
  if (typeof input !== "function") {
    throw new Error("workflow default export must be an async function");
  }
  const constructorName = input.constructor.name;
  if (constructorName !== "AsyncFunction") {
    throw new Error("workflow default export must be an async function");
  }
  return input as (ctx: unknown) => Promise<void>;
}

function pathToFileUrl(filePath: string): string {
  // Dynamic import expects file URLs; slash normalization keeps Windows paths
  // valid without relying on process-global URL helpers.
  const normalized = filePath.replace(/\\/g, "/");
  return `file://${normalized}`;
}
