import { readFile } from "node:fs/promises";
import path from "node:path";

import { createOpencodeServer } from "@opencode-ai/sdk/server";

export interface ManagedServer {
  url(): Promise<string>;
  close(): void;
}

export function createServer(
  workspaceRoot: string,
  signal: AbortSignal
): ManagedServer {
  let starting: Promise<{ url: string; close(): void }> | null = null;
  let instance: { url: string; close(): void } | null = null;

  return {
    async url(): Promise<string> {
      if (instance !== null) {
        return instance.url;
      }

      starting ??= startServer(workspaceRoot, signal);

      instance = await starting;
      return instance.url;
    },

    close(): void {
      instance?.close();
    },
  };
}

async function startServer(
  workspaceRoot: string,
  signal: AbortSignal
): Promise<{ url: string; close(): void }> {
  const config = await loadOpencodeConfig(workspaceRoot);
  return createOpencodeServer({ signal, config });
}

async function loadOpencodeConfig(
  workspaceRoot: string
): Promise<Record<string, unknown> | undefined> {
  const configPath = path.join(workspaceRoot, ".opencode", "opencode.json");
  try {
    const raw = await readFile(configPath, "utf8");
    return JSON.parse(raw) as Record<string, unknown>;
  } catch {
    return undefined;
  }
}
