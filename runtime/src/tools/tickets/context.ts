import path from "node:path";

import { type Ticket } from "./parser.ts";
import { normalizeTicketStatus } from "./schema.ts";
import { addTicketComment, updateTicketStatus } from "./ops.ts";
import { scanTickets } from "./scanner.ts";
import { TicketLookup } from "./lookup.ts";

export interface TicketContext {
  list: () => Promise<{ tickets: Ticket[]; errors: unknown[] }>;
  get: (options: {
    id: string;
  }) => Promise<{ ticket?: Ticket; error?: string }>;
  getNext: (options?: {
    status?: string;
  }) => Promise<{ ticket?: Ticket; error?: string }>;
  updateStatus: (options: {
    id: string;
    status: string;
  }) => Promise<{ ok: boolean; ticket?: Ticket; error?: string }>;
  comment: (options: {
    id: string;
    text: string;
    section?: string;
  }) => Promise<{ ok: boolean; error?: string }>;
}

function extractMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

async function buildIndex(ticketsDir: string) {
  const { tickets, errors } = await scanTickets(ticketsDir);
  return { index: new TicketLookup(tickets), errors };
}

function notFound(id: string) {
  return { ok: false as const, error: `Ticket not found: ${id}` };
}

async function wrapOp<T>(
  fn: () => Promise<T>
): Promise<T | { ok: false; error: string }> {
  try {
    return await fn();
  } catch (error: unknown) {
    return { ok: false, error: extractMessage(error) };
  }
}

export function createTicketContext(workspaceRoot: string): TicketContext {
  const ticketsDir = path.join(workspaceRoot, ".agents", "tickets");
  const getIndex = () => buildIndex(ticketsDir);

  return {
    list: async () => {
      try {
        const result = await scanTickets(ticketsDir);
        return { ok: true, ...result };
      } catch (error: unknown) {
        const msg = extractMessage(error);
        return { ok: false, tickets: [], errors: [msg], error: msg };
      }
    },
    get: ({ id }) =>
      wrapOp(async () => {
        const { index } = await getIndex();
        const ticket = index.get(id);
        return ticket === undefined
          ? notFound(id)
          : { ok: true as const, ticket };
      }),
    getNext: (options) =>
      wrapOp(async () => {
        const status = normalizeTicketStatus(options?.status ?? "OPEN");
        const { index } = await getIndex();
        return { ok: true as const, ticket: index.getNextByStatus(status) };
      }),
    updateStatus: ({ id, status }) =>
      wrapOp(async () => {
        const { index } = await getIndex();
        const ticket = index.get(id);
        if (ticket === undefined) return notFound(id);
        return {
          ok: true as const,
          ticket: await updateTicketStatus(ticket, status),
        };
      }),
    comment: ({ id, text, section }) =>
      wrapOp(async () => {
        const { index } = await getIndex();
        const ticket = index.get(id);
        if (ticket === undefined) return notFound(id);
        await addTicketComment(ticket, text, section);
        return { ok: true as const };
      }),
  };
}
