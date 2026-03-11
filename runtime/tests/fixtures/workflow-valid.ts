import type { createContext } from "../../src/context.ts";

type WorkflowContext = ReturnType<typeof createContext>;
interface WorkflowTools {
  log: { info(message: string): void };
  sleep(ms: number): Promise<void>;
}

export const meta = {
  id: "duos",
  name: "Duos",
  runtime: "host",
};

export default async function run(
  _context: WorkflowContext,
  tools: WorkflowTools
): Promise<void> {
  tools.log.info("fixture workflow started");
  await tools.sleep(5);
}
