import type { WorkflowMeta, WorkflowContext, Ticket } from "agentkata";
import { fs, tickets } from "agentkata/helpers";

export const meta: WorkflowMeta = {
  id: "pair",
  name: "Pair Workflow",
  description: "Two-agent workflow: a planner (architect+PM) and a builder (dev+QA)",
  runtime: "host",
};

const MAX_REVIEW_CYCLES = 5;
const OUTLINE_PATH = ".agents/context/OUTLINE.md";

function renderRolePrompt(template: string, ticket: Ticket): string {
  return template
    .replaceAll("{{ticket.filePath}}", ticket.filePath)
    .replaceAll("{{ ticket.filePath }}", ticket.filePath);
}

function assertOk<T extends { ok: boolean; error?: string }>(
  result: T,
  action: string,
): T {
  if (result?.ok) return result;
  throw new Error(`${action} failed: ${result?.error ?? "unknown error"}`);
}

function assertTicket(
  result: { ok: boolean; ticket?: Ticket; error?: string },
  action: string,
): Ticket {
  const checked = assertOk(result, action);
  if (!checked.ticket) throw new Error(`${action} failed: ticket not found`);
  return checked.ticket;
}

async function runPlanner(ctx: WorkflowContext) {
  const prompt = await fs.read(".agents/context/roles/planner.md");
  const result = await ctx.agent.run({
    title: "Planner: Design and create tickets",
    prompt,
  });

  if (!result.ok) {
    throw new Error(`Planner agent failed: ${result.error}`);
  }

  return result;
}

async function runBuilder(ctx: WorkflowContext, ticket: Ticket) {
  const rolePrompt = await fs.read(".agents/context/roles/builder.md");
  const result = await ctx.agent.run({
    title: `Builder: ${ticket.title}`,
    prompt: [
      renderRolePrompt(rolePrompt, ticket),
      "",
      `Ticket ID: ${ticket.ticketId}`,
      `Ticket Title: ${ticket.title}`,
      `Ticket File: ${ticket.filePath}`,
      "",
      "Ticket Description:",
      ticket.description ?? "(no description)",
    ].join("\n"),
  });

  if (!result.ok) {
    throw new Error(
      `Builder agent failed on ticket ${ticket.ticketId}: ${result.error}`,
    );
  }

  return result;
}

async function buildTicket(ctx: WorkflowContext, ticket: Ticket) {
  assertOk(
    await tickets.updateStatus({ id: ticket.ticketId, status: "IN_PROGRESS" }),
    `Update ticket ${ticket.ticketId} to IN_PROGRESS`,
  );

  for (let cycle = 1; cycle <= MAX_REVIEW_CYCLES; cycle++) {
    await runBuilder(ctx, ticket);

    const refreshed = await tickets.get({ id: ticket.ticketId });
    ticket = assertTicket(refreshed, `Refresh ticket ${ticket.ticketId}`);

    if (ticket.status === "CLOSED") {
      ctx.log.info(`Ticket ${ticket.ticketId} closed after ${cycle} cycle(s)`);
      return { ticketId: ticket.ticketId, cycles: cycle, ok: true };
    }

    if (ticket.status !== "QA_CHANGES_REQUESTED") {
      ctx.log.warn(
        `Ticket ${ticket.ticketId} has unexpected status '${ticket.status}' — stopping`,
      );
      return {
        ticketId: ticket.ticketId,
        cycles: cycle,
        ok: false,
        status: ticket.status,
      };
    }

    ctx.log.info(
      `Ticket ${ticket.ticketId} needs changes (cycle ${cycle}/${MAX_REVIEW_CYCLES})`,
    );
  }

  assertOk(
    await tickets.updateStatus({ id: ticket.ticketId, status: "STUCK" }),
    `Update ticket ${ticket.ticketId} to STUCK`,
  );
  assertOk(
    await tickets.comment({
      id: ticket.ticketId,
      text: `Stuck after ${MAX_REVIEW_CYCLES} build cycles without resolution.`,
    }),
    `Comment on stuck ticket ${ticket.ticketId}`,
  );

  return {
    ticketId: ticket.ticketId,
    cycles: MAX_REVIEW_CYCLES,
    ok: false,
    status: "STUCK",
  };
}

async function buildTickets(ctx: WorkflowContext, ticketList: Ticket[]) {
  ctx.log.info(`Building ${ticketList.length} ticket(s)`);

  const results = [];
  for (const ticket of ticketList) {
    const result = await buildTicket(ctx, ticket);
    results.push(result);
  }

  const passed = results.filter((r) => r.ok).length;
  const failed = results.filter((r) => !r.ok).length;
  ctx.log.info(`Done. ${passed} ticket(s) closed, ${failed} stuck or unresolved.`);

  return results;
}

export default async function duosWorkflow(ctx: WorkflowContext) {
  // Shortcut: if a ticket was passed directly, skip planning
  if (ctx.ticket) {
    ctx.log.info(
      `Ticket provided — skipping planner, building ticket ${ctx.ticket.ticketId}`,
    );
    const results = await buildTickets(ctx, [ctx.ticket]);
    return { ok: true, results };
  }

  // Phase 1: Planner produces outline.md and creates tickets
  ctx.log.info("Phase 1: Planner");
  await runPlanner(ctx);

  const outlineExists = await fs.exists(OUTLINE_PATH);
  if (!outlineExists) {
    throw new Error(`Planner did not produce ${OUTLINE_PATH}`);
  }

  // Phase 2: Builder processes all open tickets
  ctx.log.info("Phase 2: Builder");
  const listResult = await tickets.list();
  if (!listResult.ok) {
    throw new Error(`Failed to list tickets: ${listResult.errors}`);
  }

  const openTickets = listResult.tickets.filter((t) => t.status === "OPEN");
  const results = await buildTickets(ctx, openTickets);

  return { ok: true, results };
}
