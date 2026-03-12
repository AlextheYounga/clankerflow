import { createOpencodeClient } from "@opencode-ai/sdk";

import { normalizeServerUrl } from "./server-url.ts";
import type { AgentOptions, OpencodeClient } from "./types.ts";

export function createRuntime(options: AgentOptions): {
  client(): Promise<OpencodeClient>;
} {
  let instance: Promise<OpencodeClient> | null = null;

  return {
    async client(): Promise<OpencodeClient> {
      if (instance !== null) {
        return instance;
      }

      instance = createClient(options);
      return instance;
    },
  };
}

async function createClient(options: AgentOptions): Promise<OpencodeClient> {
  const serverUrl = await options.serverUrl();
  const baseUrl = normalizeServerUrl(serverUrl, options.runtimeEnv);
  const createClientFn = options.createClient ?? createSdkClient;
  return createClientFn(baseUrl);
}

function createSdkClient(baseUrl: string): OpencodeClient {
  return createOpencodeClient({ baseUrl });
}
