import type { createContext } from "../../src/context.ts";

type WorkflowContext = ReturnType<typeof createContext>;
interface WorkflowTools {
  log: { info(message: string): void };
  sleep(ms: number): Promise<void>;
}

export const meta = {
  id: "duos-cancel",
  name: "Duos Cancel",
  runtime: "host",
};

export default async function run(
  _context: WorkflowContext,
  tools: WorkflowTools
): Promise<void> {
  tools.log.info("waiting for cancellation");
  await tools.sleep(5_000);
}
