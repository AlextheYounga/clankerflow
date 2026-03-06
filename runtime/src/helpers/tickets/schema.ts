export type TicketStatus =
  | "OPEN"
  | "IN_PROGRESS"
  | "QA_REVIEW"
  | "QA_CHANGES_REQUESTED"
  | "STUCK"
  | "CLOSED";

export const TicketStatus = {
  OPEN: "OPEN" as const,
  IN_PROGRESS: "IN_PROGRESS" as const,
  QA_REVIEW: "QA_REVIEW" as const,
  QA_CHANGES_REQUESTED: "QA_CHANGES_REQUESTED" as const,
  STUCK: "STUCK" as const,
  CLOSED: "CLOSED" as const,
};

export function normalizeTicketStatus(
  value: string | undefined | null,
): TicketStatus {
  if (!value) return TicketStatus.OPEN;
  const raw = value.trim().toUpperCase().replace(/[-\s]/g, "_");

  const map: Record<string, TicketStatus> = {
    OPEN: TicketStatus.OPEN,
    IN_PROGRESS: TicketStatus.IN_PROGRESS,
    QA_REVIEW: TicketStatus.QA_REVIEW,
    QA_CHANGES_REQUESTED: TicketStatus.QA_CHANGES_REQUESTED,
    STUCK: TicketStatus.STUCK,
    BLOCKED: TicketStatus.STUCK,
    CLOSED: TicketStatus.CLOSED,
    DONE: TicketStatus.CLOSED,
    COMPLETE: TicketStatus.CLOSED,
    COMPLETED: TicketStatus.CLOSED,
  };

  return map[raw] || TicketStatus.OPEN;
}

export const TICKET_ID_ALIASES = ["id", "ticket_id", "ticketid"];

export function resolveTicketId(
  frontmatter: Record<string, any>,
): string | null {
  for (const key of TICKET_ID_ALIASES) {
    const value = frontmatter[key];
    if (value !== undefined && value !== null) {
      const normalized = String(value).trim();
      if (normalized) return normalized;
    }
  }
  return null;
}
