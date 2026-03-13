import type {
  WorkflowMeta,
  WorkflowContext,
  WorkflowTools,
  Ticket,
} from "clankerflow";

export const meta: WorkflowMeta = {
  id: "pair",
  name: "Pair Workflow",
  description:
    "Two-agent workflow: a planner (architect+PM) and a builder (dev+QA)",
  runtime: "host",
};

const MAX_REVIEW_CYCLES = 5;
const OUTLINE_PATH = ".agents/context/OUTLINE.md";

async function runPlanner(tools: WorkflowTools) {
  const prompt = await tools.fs.read(".agents/context/roles/planner.md");
  const result = await tools.agent.run({
    title: "Planner: Design and create tickets",
    prompt,
  });
  if (!result.ok) throw new Error(`Planner agent failed: ${result.error}`);
}

async function runBuilder(tools: WorkflowTools, ticket: Ticket) {
  const rolePrompt = await tools.fs.read(".agents/context/roles/builder.md");
  const prompt = rolePrompt.replaceAll("{{ticket.filePath}}", ticket.filePath);
  const result = await tools.agent.run({
    title: `Builder: ${ticket.title}`,
    prompt,
  });
  if (!result.ok)
    throw new Error(
      `Builder agent failed on ticket ${ticket.ticketId}: ${result.error}`
    );
}

async function buildTicket(tools: WorkflowTools, initial: Ticket) {
  await tools.tickets.updateStatus({
    id: initial.ticketId,
    status: "IN_PROGRESS",
  });

  let ticket = initial;
  for (let cycle = 1; cycle <= MAX_REVIEW_CYCLES; cycle++) {
    await runBuilder(tools, ticket);

    const refreshed = await tools.tickets.get({ id: ticket.ticketId });
    if (!refreshed.ticket)
      throw new Error(`Refresh ticket ${ticket.ticketId} failed`);
    ticket = refreshed.ticket;

    if (ticket.status === "CLOSED") {
      tools.log.info(
        `Ticket ${ticket.ticketId} closed after ${cycle} cycle(s)`
      );
      return { ticketId: ticket.ticketId, cycles: cycle, ok: true };
    }

    if (ticket.status !== "QA_CHANGES_REQUESTED") {
      tools.log.warn(
        `Ticket ${ticket.ticketId} has unexpected status '${ticket.status}' — stopping`
      );
      return {
        ticketId: ticket.ticketId,
        cycles: cycle,
        ok: false,
        status: ticket.status,
      };
    }

    tools.log.info(
      `Ticket ${ticket.ticketId} needs changes (cycle ${cycle}/${MAX_REVIEW_CYCLES})`
    );
  }

  tools.log.warn(
    `Ticket ${ticket.ticketId} unresolved after ${MAX_REVIEW_CYCLES} cycle(s)`
  );
  return {
    ticketId: ticket.ticketId,
    cycles: MAX_REVIEW_CYCLES,
    ok: false,
    status: "QA_CHANGES_REQUESTED",
  };
}

export default async function duosWorkflow(
  context: WorkflowContext,
  tools: WorkflowTools
) {
  // Shortcut: if a ticket was passed directly, skip planning
  if (context.ticket) {
    tools.log.info(
      `Ticket provided — skipping planner, building ticket ${context.ticket.ticketId}`
    );
    const branchName =
      context.ticket.branch ?? `ticket-${context.ticket.ticketId}`;
    await tools.git.checkoutBranch(branchName, "master");
    const result = await buildTicket(tools, context.ticket);
    return { ok: true, results: [result] };
  }

  // Phase 1: Planner produces outline and creates tickets
  tools.log.info("Phase 1: Planner");
  await runPlanner(tools);

  const hasOutline = await tools.fs.exists(OUTLINE_PATH);
  if (!hasOutline) throw new Error(`Planner did not produce ${OUTLINE_PATH}`);

  // Phase 2: Builder processes all open tickets
  tools.log.info("Phase 2: Builder");
  const { tickets } = await tools.tickets.list();
  const openTickets = tickets.filter((t) => t.status === "OPEN");
  tools.log.info(`Building ${openTickets.length} ticket(s)`);

  const results = [];
  for (const ticket of openTickets) {
    const branchName = ticket.branch ?? `ticket-${ticket.ticketId}`;
    await tools.git.checkoutBranch(branchName, "master");
    results.push(await buildTicket(tools, ticket));
  }

  const passed = results.filter((r) => r.ok).length;
  const failed = results.filter((r) => !r.ok).length;
  tools.log.info(`Done. ${passed} ticket(s) closed, ${failed} unresolved.`);

  return { ok: true, results };
}
