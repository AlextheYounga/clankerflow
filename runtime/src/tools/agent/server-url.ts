import type { RuntimeEnv } from "../../protocol.ts";

export function normalizeServerUrl(url: string, runtimeEnv: RuntimeEnv): string {
  if (runtimeEnv !== "container") {
    return url;
  }

  try {
    const parsed = new URL(url);
    if (parsed.hostname === "127.0.0.1" || parsed.hostname === "localhost") {
      parsed.hostname = "host.docker.internal";
    }
    return parsed.toString().replace(/\/$/, "");
  } catch {
    return url;
  }
}
