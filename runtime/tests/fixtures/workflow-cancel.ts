export const meta = {
  id: "duos-cancel",
  name: "Duos Cancel",
  runtime: "host",
};

export default async function run(ctx: any): Promise<void> {
  ctx.log.info("waiting for cancellation");
  await ctx.sleep(5_000);
}
