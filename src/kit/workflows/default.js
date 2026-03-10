import { tickets } from "../.agentctl/lib/helpers.js";

export const meta = {
  id: "default",
  name: "Default Workflow",
  description: "Process the next open ticket",
  runtime: "host",
};

export default async function defaultWorkflow(ctx) {
  const next = await tickets.getNext({ status: "OPEN" });

  if (!next.ok) {
    console.log('No more tickets to process.');
  }

  if (!next.ticket) {
    ctx.log.info("No open tickets found");
    return { ok: true, skipped: true };
  }

  const ticket = next.ticket;
  ctx.log.info(`Processing ticket ${ticket.ticketId}: ${ticket.title}`);

  (await tickets.updateStatus({
    id: ticket.ticketId,
    status: "IN_PROGRESS",
  }),
    `Update ticket ${ticket.ticketId} to IN_PROGRESS`);

  const result = await ctx.agent.run({
    title: `Work on: ${ticket.title}`,
    prompt: `You have been assigned this ticket:\n\nTitle: ${ticket.title}\n${ticket.description ?? ""}\n\nImplement what is described. When done, summarize what you did.`,
  });

  if (!result.ok) {
    throw new Error(
      `Agent failed on ticket ${ticket.ticketId}: ${result.error}`,
    );
  }

  await tickets.comment({
    id: ticket.ticketId,
    text: result.output || "Work completed.",
  });

  await tickets.updateStatus({
    id: ticket.ticketId,
    status: "QA_REVIEW",
  });

  ctx.log.info(`Ticket ${ticket.ticketId} moved to QA_REVIEW`);
  return { ok: true, ticketId: ticket.ticketId };
}
