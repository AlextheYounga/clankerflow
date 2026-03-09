import type { createContext } from "../../src/context.ts";

type WorkflowContext = ReturnType<typeof createContext>;

export const meta = {
  id: "duos",
  name: "Duos",
  runtime: "host",
};

export default async function run(ctx: WorkflowContext): Promise<void> {
  ctx.log.info("fixture workflow started");
  await ctx.sleep(5);
}
