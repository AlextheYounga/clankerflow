import type {
  WorkflowMeta,
  WorkflowContext,
  WorkflowTools,
  Ticket,
} from "agentkata";

export const meta: WorkflowMeta = {
  id: "dev-team",
  name: "Dev Team Workflow",
  description: "Architect plans, PM creates tickets, dev+QA iterate to completion",
  runtime: "host",
};

const MAX_REVIEW_CYCLES = 5;
const OUTLINE_PATH = ".agents/context/OUTLINE.md";

function renderRolePrompt(template: string, ticket: Ticket): string {
  return template.replaceAll("{{ticket.filePath}}", ticket.filePath);
}

async function runArchitect(tools: WorkflowTools) {
  const prompt = await tools.fs.read(".agents/context/roles/architect.md");
  const result = await tools.agent.run({ title: "Architect: Create outline", prompt });
  if (!result.ok) throw new Error(`Architect agent failed: ${result.error}`);
}

async function runProjectManager(tools: WorkflowTools, outlineContent: string) {
  const rolePrompt = await tools.fs.read(".agents/context/roles/pm.md");
  const result = await tools.agent.run({
    title: "PM: Create tickets",
    prompt: [rolePrompt, "", "Architecture Outline:", outlineContent].join("\n"),
  });
  if (!result.ok) throw new Error(`PM agent failed: ${result.error}`);
}

async function runDev(tools: WorkflowTools, ticket: Ticket) {
  const rolePrompt = await tools.fs.read(".agents/context/roles/dev.md");
  const prompt = renderRolePrompt(rolePrompt, ticket);
  const result = await tools.agent.run({ title: `Dev: ${ticket.title}`, prompt });
  if (!result.ok) throw new Error(`Dev agent failed on ticket ${ticket.ticketId}: ${result.error}`);
  return result;
}

async function runQA(tools: WorkflowTools, ticket: Ticket, devOutput: string | undefined) {
  const rolePrompt = await tools.fs.read(".agents/context/roles/qa.md");
  const prompt = renderRolePrompt(rolePrompt, ticket);
  const result = await tools.agent.run({ title: `QA: ${ticket.title}`, prompt: [prompt, "", devOutput ?? "(no output provided)"].join("\n") });
  if (!result.ok) throw new Error(`QA agent failed on ticket ${ticket.ticketId}: ${result.error}`);
}

async function processDevQaCycle(tools: WorkflowTools, ticket: Ticket) {
  await tools.tickets.updateStatus({ id: ticket.ticketId, status: "IN_PROGRESS" });
  const devResult = await runDev(tools, ticket);
  await tools.tickets.updateStatus({ id: ticket.ticketId, status: "QA_REVIEW" });
  await runQA(tools, ticket, devResult.output);
  const refreshed = await tools.tickets.get({ id: ticket.ticketId });
  if (!refreshed.ticket) throw new Error(`Refresh ticket ${ticket.ticketId} failed`);
  return refreshed.ticket;
}

async function passTicketToDevTeam(tools: WorkflowTools, ticket: Ticket) {
  for (let cycle = 1; cycle <= MAX_REVIEW_CYCLES; cycle++) {
    ticket = await processDevQaCycle(tools, ticket);

    if (ticket.status === "CLOSED") {
      tools.log.info(`Ticket ${ticket.ticketId} closed after ${cycle} cycle(s)`);
      return { ticketId: ticket.ticketId, cycles: cycle, ok: true };
    }

    if (ticket.status !== "QA_CHANGES_REQUESTED") {
      tools.log.warn(`Ticket ${ticket.ticketId} has unexpected status '${ticket.status}' after QA — stopping`);
      return { ticketId: ticket.ticketId, cycles: cycle, ok: false, status: ticket.status };
    }

    tools.log.info(`Ticket ${ticket.ticketId} needs changes (cycle ${cycle}/${MAX_REVIEW_CYCLES})`);
  }

  await tools.tickets.updateStatus({ id: ticket.ticketId, status: "STUCK" });
  await tools.tickets.comment({
    id: ticket.ticketId,
    text: `Stuck after ${MAX_REVIEW_CYCLES} dev/QA cycles without resolution.`,
  });

  return { ticketId: ticket.ticketId, cycles: MAX_REVIEW_CYCLES, ok: false, status: "STUCK" };
}

export default async function squadWorkflow(_context: WorkflowContext, tools: WorkflowTools) {
  // Phase 1: Architect produces outline.md
  tools.log.info("Phase 1: Architect");
  await runArchitect(tools);

  const hasOutline = await tools.fs.exists(OUTLINE_PATH);
  if (!hasOutline) throw new Error(`Architect did not produce ${OUTLINE_PATH}`);

  const outline = await tools.fs.read(OUTLINE_PATH);

  // Phase 2: PM reads outline and creates tickets
  tools.log.info("Phase 2: Project Manager");
  await runProjectManager(tools, outline);

  // Phase 3: Dev + QA loop through all open tickets
  tools.log.info("Phase 3: Dev + QA");
  const { tickets } = await tools.tickets.list();
  const openTickets = tickets.filter((t) => t.status === "OPEN");
  tools.log.info(`Processing ${openTickets.length} open ticket(s)`);

  const results = [];
  for (const ticket of openTickets) {
    results.push(await passTicketToDevTeam(tools, ticket));
  }

  const passed = results.filter((r) => r.ok).length;
  const failed = results.filter((r) => !r.ok).length;
  tools.log.info(`Done. ${passed} ticket(s) closed, ${failed} stuck or unresolved.`);

  return { ok: true, results };
}
