export { createTicketContext, type TicketContext } from "./tickets/context.ts";
import type { Ticket } from "./tickets/parser.ts";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isValidTicketStatus(status: unknown): boolean {
  return (
    status === "OPEN" ||
    status === "IN_PROGRESS" ||
    status === "QA_REVIEW" ||
    status === "QA_CHANGES_REQUESTED" ||
    status === "STUCK" ||
    status === "CLOSED"
  );
}

function isOptionalString(value: unknown): value is string | null | undefined {
  return value === undefined || value === null || typeof value === "string";
}

export function isTicket(value: unknown): value is Ticket {
  if (!isRecord(value)) return false;

  return (
    typeof value.ticketId === "string" &&
    typeof value.title === "string" &&
    isValidTicketStatus(value.status) &&
    isOptionalString(value.branch) &&
    typeof value.worktree === "string" &&
    (value.description === null || typeof value.description === "string") &&
    typeof value.filePath === "string" &&
    isRecord(value.frontmatter)
  );
}

function normalizeBranch(ticket: Ticket): string | null {
  if (typeof ticket.branch === "string" && ticket.branch.trim().length > 0) {
    return ticket.branch.trim();
  }

  const worktree = ticket.worktree.trim();
  if (worktree.length > 0 && worktree !== "none") {
    return worktree;
  }

  return null;
}

export function toContextTicket(ticket: unknown): Ticket | null {
  if (!isTicket(ticket)) return null;

  return {
    ...ticket,
    branch: normalizeBranch(ticket),
  };
}
