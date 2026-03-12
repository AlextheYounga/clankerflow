import type {
  WorkflowMeta,
  WorkflowContext,
  WorkflowTools,
  Ticket,
} from "agentkata";

export const meta: WorkflowMeta = {
  id: "pair",
  name: "Pair Workflow",
  description: "Two-agent workflow: a planner (architect+PM) and a builder (dev+QA)",
  runtime: "host",
};

const MAX_REVIEW_CYCLES = 5;
const OUTLINE_PATH = ".agents/context/OUTLINE.md";

function renderRolePrompt(template: string, ticket: Ticket): string {
  return template.replaceAll("{{ticket.filePath}}", ticket.filePath);
}

async function runPlanner(tools: WorkflowTools) {
  const prompt = await tools.fs.read(".agents/context/roles/planner.md");
  const result = await tools.agent.run({
    title: "Planner: Design and create tickets",
    prompt,
  });

  if (!result.ok) {
    throw new Error(`Planner agent failed: ${result.error}`);
  }

  return result;
}

async function runBuilder(tools: WorkflowTools, ticket: Ticket) {
  const rolePrompt = await tools.fs.read(".agents/context/roles/builder.md");
  const updatedPrompt = renderRolePrompt(rolePrompt, ticket);
  const result = await tools.agent.run({
    title: `Builder: ${ticket.title}`,
    prompt: updatedPrompt,
  });

  if (!result.ok) {
    throw new Error(
      `Builder agent failed on ticket ${ticket.ticketId}: ${result.error}`,
    );
  }

  return result;
}

export default async function duosWorkflow(
  context: WorkflowContext,
  tools: WorkflowTools,
) {
  // Shortcut: if a ticket was passed directly, skip planning
  if (context.ticket) {
    tools.log.info(
      `Ticket provided — skipping planner, building ticket ${context.ticket.ticketId}`,
    );
    let ticket = context.ticket;
    const branchName = ticket.branch ?? `ticket-${ticket.ticketId}`;
    await tools.git.checkoutBranch(branchName, "master");

    const inProgress = await tools.tickets.updateStatus({
      id: ticket.ticketId,
      status: "IN_PROGRESS",
    });
    if (!inProgress.ok) {
      throw new Error(
        `Update ticket ${ticket.ticketId} to IN_PROGRESS failed: ${inProgress.error ?? "unknown error"}`,
      );
    }

    let cycle = 0;
    while (true) {
      cycle += 1;
      if (cycle > MAX_REVIEW_CYCLES) break;
      await runBuilder(tools, ticket);

      const refreshed = await tools.tickets.get({ id: ticket.ticketId });
      if (!refreshed.ok) {
        throw new Error(
          `Refresh ticket ${ticket.ticketId} failed: ${refreshed.error ?? "unknown error"}`,
        );
      }
      if (!refreshed.ticket) {
        throw new Error(
          `Refresh ticket ${ticket.ticketId} failed: ticket not found`,
        );
      }
      ticket = refreshed.ticket;

      if (ticket.status === "CLOSED") {
        tools.log.info(
          `Ticket ${ticket.ticketId} closed after ${cycle} cycle(s)`,
        );
        return {
          ok: true,
          results: [{ ticketId: ticket.ticketId, cycles: cycle, ok: true }],
        };
      }

      if (ticket.status !== "QA_CHANGES_REQUESTED") {
        tools.log.warn(
          `Ticket ${ticket.ticketId} has unexpected status '${ticket.status}' — stopping`,
        );
        return {
          ok: true,
          results: [
            {
              ticketId: ticket.ticketId,
              cycles: cycle,
              ok: false,
              status: ticket.status,
            },
          ],
        };
      }

      tools.log.info(
        `Ticket ${ticket.ticketId} needs changes (cycle ${cycle}/${MAX_REVIEW_CYCLES})`,
      );
    }

    tools.log.warn(
      `Ticket ${ticket.ticketId} unresolved after ${MAX_REVIEW_CYCLES} cycle(s)`,
    );
    return {
      ok: true,
      results: [
        {
          ticketId: ticket.ticketId,
          cycles: MAX_REVIEW_CYCLES,
          ok: false,
          status: "QA_CHANGES_REQUESTED",
        },
      ],
    };
  }

  // Phase 1: Planner produces outline.md and creates tickets
  tools.log.info("Phase 1: Planner");
  await runPlanner(tools);

  const outlineExists = await tools.fs.exists(OUTLINE_PATH);
  if (!outlineExists) {
    throw new Error(`Planner did not produce ${OUTLINE_PATH}`);
  }

  // Phase 2: Builder processes all open tickets
  tools.log.info("Phase 2: Builder");
  const listResult = await tools.tickets.list();
  if (!listResult.ok) {
    throw new Error(`Failed to list tickets: ${listResult.errors}`);
  }

  const openTickets = listResult.tickets.filter((t) => t.status === "OPEN");
  tools.log.info(`Building ${openTickets.length} ticket(s)`);

  const results = [];
  for (const ticket of openTickets) {
    let workingTicket = ticket;
    const branchName = workingTicket.branch ?? `ticket-${workingTicket.ticketId}`;
    await tools.git.checkoutBranch(branchName, "master");

    const inProgress = await tools.tickets.updateStatus({
      id: workingTicket.ticketId,
      status: "IN_PROGRESS",
    });
    if (!inProgress.ok) {
      throw new Error(
        `Update ticket ${workingTicket.ticketId} to IN_PROGRESS failed: ${inProgress.error ?? "unknown error"}`,
      );
    }

    let result = {
      ticketId: workingTicket.ticketId,
      cycles: 0,
      ok: false,
      status: "QA_CHANGES_REQUESTED",
    } as {
      ticketId: string;
      cycles: number;
      ok: boolean;
      status?: string;
    };

    let cycle = 0;
    while (true) {
      cycle += 1;
      if (cycle > MAX_REVIEW_CYCLES) break;
      await runBuilder(tools, workingTicket);

      const refreshed = await tools.tickets.get({ id: workingTicket.ticketId });
      if (!refreshed.ok) {
        throw new Error(
          `Refresh ticket ${workingTicket.ticketId} failed: ${refreshed.error ?? "unknown error"}`,
        );
      }
      if (!refreshed.ticket) {
        throw new Error(
          `Refresh ticket ${workingTicket.ticketId} failed: ticket not found`,
        );
      }
      workingTicket = refreshed.ticket;

      if (workingTicket.status === "CLOSED") {
        tools.log.info(
          `Ticket ${workingTicket.ticketId} closed after ${cycle} cycle(s)`,
        );
        result = { ticketId: workingTicket.ticketId, cycles: cycle, ok: true };
        break;
      }

      if (workingTicket.status !== "QA_CHANGES_REQUESTED") {
        tools.log.warn(
          `Ticket ${workingTicket.ticketId} has unexpected status '${workingTicket.status}' — stopping`,
        );
        result = {
          ticketId: workingTicket.ticketId,
          cycles: cycle,
          ok: false,
          status: workingTicket.status,
        };
        break;
      }

      tools.log.info(
        `Ticket ${workingTicket.ticketId} needs changes (cycle ${cycle}/${MAX_REVIEW_CYCLES})`,
      );
    }

    if (result.cycles === 0) {
      tools.log.warn(
        `Ticket ${workingTicket.ticketId} unresolved after ${MAX_REVIEW_CYCLES} cycle(s)`,
      );
      result = {
        ticketId: workingTicket.ticketId,
        cycles: MAX_REVIEW_CYCLES,
        ok: false,
        status: "QA_CHANGES_REQUESTED",
      };
    }

    results.push(result);
  }

  const passed = results.filter((r) => r.ok).length;
  const failed = results.filter((r) => !r.ok).length;
  tools.log.info(
    `Done. ${passed} ticket(s) closed, ${failed} unresolved.`,
  );

  return { ok: true, results };
}
