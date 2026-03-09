import type { createContext } from "../../src/context.ts";

type WorkflowContext = ReturnType<typeof createContext>;

export const meta = {
  id: "duos-cancel",
  name: "Duos Cancel",
  runtime: "host",
};

export default async function run(ctx: WorkflowContext): Promise<void> {
  ctx.log.info("waiting for cancellation");
  await ctx.sleep(5_000);
}
