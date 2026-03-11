import type { WorkflowMeta, WorkflowContext, WorkflowTools } from "agentkata";

export const meta: WorkflowMeta = {
  id: "default",
  name: "Default Workflow",
  description: "Process the next open ticket",
  runtime: "host",
};

export default async function defaultWorkflow(
  ctx: WorkflowContext,
  tools: WorkflowTools,
) {
  const { agent, log, tickets } = tools;
  let next = await tickets.getNext({ status: "OPEN" });
  if (ctx.ticket) next = { ok: true, ticket: ctx.ticket };

  if (!next?.ticket) {
    log.info("No open tickets found");
    return { ok: true, skipped: true };
  }

  const ticket = next.ticket;

  log.info(`Processing ticket ${ticket.ticketId}: ${ticket.title}`);

  await tickets.updateStatus({ id: ticket.ticketId, status: "IN_PROGRESS" });

  const result = await agent.run({
    title: `Work on: ${ticket.title}`,
    prompt: `You have been assigned this ticket:\n\nTitle: ${ticket.title}\n${ticket.description ?? ""}\n\nImplement what is described. When done, summarize what you did.`,
  });

  if (!result.ok) {
    throw new Error(`Agent failed on ticket ${ticket.ticketId}: ${result.error}`);
  }

  await tickets.comment({
    id: ticket.ticketId,
    text: result.output || "Work completed.",
  });

  await tickets.updateStatus({
    id: ticket.ticketId,
    status: "QA_REVIEW",
  });

  log.info(`Ticket ${ticket.ticketId} moved to QA_REVIEW`);
  return { ok: true, ticketId: ticket.ticketId };
}
