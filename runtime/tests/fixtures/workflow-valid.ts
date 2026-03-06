export const meta = {
  id: "duos",
  name: "Duos",
  runtime: "host",
};

export default async function run(ctx: any): Promise<void> {
  ctx.log.info("fixture workflow started");
  await ctx.sleep(5);
}
