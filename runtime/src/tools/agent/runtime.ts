import { readFile } from "node:fs/promises";
import path from "node:path";

import { createOpencodeClient } from "@opencode-ai/sdk";

import { normalizeServerUrl } from "./server-url.ts";
import type { AgentOptions, OpenCodeClient, RuntimeSettings } from "./types.ts";

const DEFAULT_OPENCODE_URL = "http://127.0.0.1:4096";

export function createRuntime(options: AgentOptions): {
  client(): Promise<OpenCodeClient>;
} {
  let instance: Promise<OpenCodeClient> | null = null;

  return {
    async client(): Promise<OpenCodeClient> {
      if (instance !== null) {
        return instance;
      }

      instance = createClient(options);
      return instance;
    },
  };
}

async function createClient(options: AgentOptions): Promise<OpenCodeClient> {
  const loadSettings = options.loadSettings ?? loadRuntimeSettings;
  const settings = await loadSettings(options.workspaceRoot);
  const serverUrl = resolveServerUrl(settings);
  const baseUrl = normalizeServerUrl(serverUrl, options.runtimeEnv);
  const createClientFn = options.createClient ?? createSdkClient;
  return createClientFn(baseUrl);
}

function resolveServerUrl(settings: RuntimeSettings): string {
  if (typeof settings.opencode?.server_url === "string") {
    return settings.opencode.server_url;
  }

  return DEFAULT_OPENCODE_URL;
}

async function loadRuntimeSettings(workspaceRoot: string): Promise<RuntimeSettings> {
  const settingsPath = path.join(workspaceRoot, ".agents", "settings.json");
  const raw = await readFile(settingsPath, "utf8");
  return JSON.parse(raw) as RuntimeSettings;
}

function createSdkClient(baseUrl: string): OpenCodeClient {
  return createOpencodeClient({ baseUrl }) as unknown as OpenCodeClient;
}
