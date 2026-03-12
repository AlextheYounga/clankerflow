import type { createContext } from "../../src/context.ts";

type WorkflowContext = ReturnType<typeof createContext>;
interface WorkflowTools {
  log: { info(message: string): void };
}

export const meta = {
  id: "fetch-error",
  name: "Fetch Error",
  runtime: "host",
};

// Simulates the exact failure path in duos.ts when agent.run returns ok:false
// because the OpenCode server is unreachable (e.g. "fetch failed").
export default async function run(
  _context: WorkflowContext,
  _tools: WorkflowTools
): Promise<void> {
  throw new Error("Planner agent failed: fetch failed");
}
