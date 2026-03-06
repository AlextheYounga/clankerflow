import path from "node:path";

export type WorkflowMeta = {
  id: string;
  name: string;
  runtime: "host" | "container";
};

export type WorkflowModule = {
  meta: WorkflowMeta;
  run: (ctx: unknown) => Promise<void>;
};

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
  if (!input || typeof input !== "object") {
    throw new Error("workflow meta export is required");
  }
  const meta = input as Partial<WorkflowMeta>;
  if (!meta.id || typeof meta.id !== "string") {
    throw new Error("workflow meta.id must be a non-empty string");
  }
  if (!meta.name || typeof meta.name !== "string") {
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
  const runFn = input as (ctx: unknown) => unknown;
  const constructorName = runFn.constructor?.name;
  if (constructorName !== "AsyncFunction") {
    throw new Error("workflow default export must be an async function");
  }
  return input as (ctx: unknown) => Promise<void>;
}

function pathToFileUrl(filePath: string): string {
  const normalized = filePath.replace(/\\/g, "/");
  return `file://${normalized}`;
}
