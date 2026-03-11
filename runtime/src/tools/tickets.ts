export { createTicketContext, type TicketContext } from "./tickets/context.ts";
import type { Ticket } from "./tickets/parser.ts";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function isTicket(value: unknown): value is Ticket {
  if (!isRecord(value)) return false;

  const status = value.status;
  const validStatus =
    status === "OPEN" ||
    status === "IN_PROGRESS" ||
    status === "QA_REVIEW" ||
    status === "QA_CHANGES_REQUESTED" ||
    status === "STUCK" ||
    status === "CLOSED";

  return (
    typeof value.ticketId === "string" &&
    typeof value.title === "string" &&
    validStatus &&
    typeof value.worktree === "string" &&
    (value.description === null || typeof value.description === "string") &&
    typeof value.filePath === "string" &&
    isRecord(value.frontmatter)
  );
}

export function toContextTicket(ticket: unknown): Ticket | null {
  return isTicket(ticket) ? ticket : null;
}