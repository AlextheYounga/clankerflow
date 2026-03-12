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
  const { agent, log, tickets, git, fs } = tools;
  // Prefer the next open ticket, but allow the caller to pin a specific ticket.
  let next = await tickets.getNext({ status: "OPEN" });
  if (ctx.ticket) next = { ok: true, ticket: ctx.ticket };

  // Exit early when there is nothing to process.
  if (!next.ticket) {
    log.info("No open tickets found");
    return { ok: true, skipped: true };
  }

  // Ensure work happens on a per-ticket branch with a stable fallback name.
  const ticket = next.ticket;
  const branchName = ticket.branch ?? "ticket-" + ticket.ticketId;
  await git.checkoutBranch(branchName, "master");

  log.info(`Processing ticket ${ticket.ticketId}: ${ticket.title}`);

  // Transition to in-progress before delegating to the agent.
  await tickets.updateStatus({ id: ticket.ticketId, status: "IN_PROGRESS" });

  // Provide the agent with the ticket context and a concise completion request.
  const prompt = await fs.read("src/kit/context/roles/builder.md");
  const updatedPrompt = prompt.replaceAll(`{{ticket.filePath}}`, ticket.filePath);

  const result = await agent.run({
    title: ticket.title,
    prompt: updatedPrompt,
  });

  // Bubble up failures so the workflow can be retried or handled upstream.
  if (!result.ok) {
    throw new Error(`Agent failed on ticket ${ticket.ticketId}: ${result.error}`);
  }

  // Record the agent output as a ticket comment for traceability.
  await tickets.comment({
    id: ticket.ticketId,
    text: result.output ?? "Work completed.",
  });

  // Move to QA review after successful completion and reporting.
  await tickets.updateStatus({
    id: ticket.ticketId,
    status: "QA_REVIEW",
  });

  log.info(`Ticket ${ticket.ticketId} moved to QA_REVIEW`);
  return { ok: true, ticketId: ticket.ticketId };
}
