import type {
  WorkflowMeta,
  WorkflowContext,
  WorkflowTools,
  Ticket,
} from "agentkata"

export const meta: WorkflowMeta = {
  id: "dev-team",
  name: "Dev Team Workflow",
  description: "Architect plans, PM creates tickets, dev+QA iterate to completion",
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

async function runArchitect(tools: WorkflowTools) {
  const prompt = await tools.fs.read(".agents/context/roles/architect.md");
  const result = await tools.agent.run({
    title: "Architect: Create outline",
    prompt,
  });

  if (!result.ok) {
    throw new Error(`Architect agent failed: ${result.error}`);
  }

  return result;
}

async function runProjectManager(
  tools: WorkflowTools,
  outlineContent: string,
) {
  const rolePrompt = await tools.fs.read(".agents/context/roles/pm.md");
  const result = await tools.agent.run({
    title: "PM: Create tickets",
    prompt: [rolePrompt, "", "Architecture Outline:", outlineContent].join("\n"),
  });

  if (!result.ok) {
    throw new Error(`PM agent failed: ${result.error}`);
  }
}

async function runDev(tools: WorkflowTools, ticket: Ticket) {
  const rolePrompt = await tools.fs.read(".agents/context/roles/dev.md");
  const result = await tools.agent.run({
    title: `Dev: ${ticket.title}`,
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
      `Dev agent failed on ticket ${ticket.ticketId}: ${result.error}`,
    );
  }

  return result;
}

async function runQA(
  tools: WorkflowTools,
  ticket: Ticket,
  devOutput: string | undefined,
) {
  const rolePrompt = await tools.fs.read(".agents/context/roles/qa.md");
  const result = await tools.agent.run({
    title: `QA: ${ticket.title}`,
    prompt: [
      renderRolePrompt(rolePrompt, ticket),
      "",
      `Ticket ID: ${ticket.ticketId}`,
      `Ticket Title: ${ticket.title}`,
      `Ticket File: ${ticket.filePath}`,
      "",
      "Ticket Description:",
      ticket.description ?? "(no description)",
      "",
      "Implementation to review:",
      devOutput ?? "(no output provided)",
    ].join("\n"),
  });

  if (!result.ok) {
    throw new Error(
      `QA agent failed on ticket ${ticket.ticketId}: ${result.error}`,
    );
  }
}

async function processDevQaCycle(tools: WorkflowTools, ticket: Ticket) {
  assertOk(
    await tools.tickets.updateStatus({ id: ticket.ticketId, status: "IN_PROGRESS" }),
    `Update ticket ${ticket.ticketId} to IN_PROGRESS`,
  );

  const devResult = await runDev(tools, ticket);

  assertOk(
    await tools.tickets.updateStatus({ id: ticket.ticketId, status: "QA_REVIEW" }),
    `Update ticket ${ticket.ticketId} to QA_REVIEW`,
  );

  await runQA(tools, ticket, devResult.output);

  const refreshed = await tools.tickets.get({ id: ticket.ticketId });
  return assertTicket(refreshed, `Refresh ticket ${ticket.ticketId}`);
}

async function passTicketToDevTeam(tools: WorkflowTools, ticket: Ticket) {
  for (let cycle = 1; cycle <= MAX_REVIEW_CYCLES; cycle++) {
    ticket = await processDevQaCycle(tools, ticket);

    if (ticket.status === "CLOSED") {
      tools.log.info(`Ticket ${ticket.ticketId} closed after ${cycle} cycle(s)`);
      return { ticketId: ticket.ticketId, cycles: cycle, ok: true };
    }

    if (ticket.status !== "QA_CHANGES_REQUESTED") {
      tools.log.warn(
        `Ticket ${ticket.ticketId} has unexpected status '${ticket.status}' after QA — stopping`,
      );
      return {
        ticketId: ticket.ticketId,
        cycles: cycle,
        ok: false,
        status: ticket.status,
      };
    }

    tools.log.info(
      `Ticket ${ticket.ticketId} needs changes (cycle ${cycle}/${MAX_REVIEW_CYCLES})`,
    );
  }

  assertOk(
    await tools.tickets.updateStatus({ id: ticket.ticketId, status: "STUCK" }),
    `Update ticket ${ticket.ticketId} to STUCK`,
  );
  assertOk(
    await tools.tickets.comment({
      id: ticket.ticketId,
      text: `Stuck after ${MAX_REVIEW_CYCLES} dev/QA cycles without resolution.`,
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

export default async function squadWorkflow(
  _context: WorkflowContext,
  tools: WorkflowTools,
) {
  // Phase 1: Architect produces outline.md
  tools.log.info("Phase 1: Architect");
  await runArchitect(tools);

  const outlineExists = await tools.fs.exists(OUTLINE_PATH);
  if (!outlineExists) {
    throw new Error(`Architect did not produce ${OUTLINE_PATH}`);
  }

  const outline = await tools.fs.read(OUTLINE_PATH);

  // Phase 2: PM reads outline and creates tickets
  tools.log.info("Phase 2: Project Manager");
  await runProjectManager(tools, outline);

  // Phase 3: Dev + QA loop through all open tickets
  tools.log.info("Phase 3: Dev + QA");
  const listResult = await tools.tickets.list();
  if (!listResult.ok) {
    throw new Error(`Failed to list tickets: ${listResult.errors}`);
  }

  const openTickets = listResult.tickets.filter((t) => t.status === "OPEN");
  tools.log.info(`Processing ${openTickets.length} open ticket(s)`);

  const results = [];
  for (const ticket of openTickets) {
    const result = await passTicketToDevTeam(tools, ticket);
    results.push(result);
  }

  const passed = results.filter((r) => r.ok).length;
  const failed = results.filter((r) => !r.ok).length;
  tools.log.info(
    `Done. ${passed} ticket(s) closed, ${failed} stuck or unresolved.`,
  );

  return { ok: true, results };
}
