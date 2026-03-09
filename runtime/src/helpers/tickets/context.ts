import path from "node:path";

import { type Ticket } from "./parser.ts";
import { normalizeTicketStatus } from "./schema.ts";
import { addTicketComment, updateTicketStatus } from "./ops.ts";
import { scanTickets } from "./scanner.ts";
import { TicketLookup } from "./lookup.ts";

export interface TicketContext {
  list: () => Promise<{ ok: boolean; tickets: Ticket[]; errors: unknown[] }>;
  get: (options: {
    id: string;
  }) => Promise<{ ok: boolean; ticket?: Ticket; error?: string }>;
  getNext: (options?: {
    status?: string;
  }) => Promise<{ ok: boolean; ticket?: Ticket; error?: string }>;
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

export function createTicketContext(workspaceRoot: string): TicketContext {
  const ticketsDir = path.join(workspaceRoot, ".agents", "tickets");

  async function getIndex() {
    const { tickets, errors } = await scanTickets(ticketsDir);
    return { index: new TicketLookup(tickets), errors };
  }

  return {
    list: async () => {
      try {
        const { tickets, errors } = await scanTickets(ticketsDir);
        return { ok: true, tickets, errors };
      } catch (error: unknown) {
        return {
          ok: false,
          tickets: [],
          errors: [extractMessage(error)],
          error: extractMessage(error),
        };
      }
    },
    get: async ({ id }) => {
      try {
        const { index } = await getIndex();
        const ticket = index.get(id);
        if (ticket === undefined) return { ok: false, error: `Ticket not found: ${id}` };
        return { ok: true, ticket };
      } catch (error: unknown) {
        return { ok: false, error: extractMessage(error) };
      }
    },
    getNext: async (options) => {
      try {
        const status = normalizeTicketStatus(options?.status ?? "OPEN");
        const { index } = await getIndex();
        const ticket = index.getNextByStatus(status);
        return { ok: true, ticket };
      } catch (error: unknown) {
        return { ok: false, error: extractMessage(error) };
      }
    },
    updateStatus: async ({ id, status }) => {
      try {
        const { index } = await getIndex();
        const ticket = index.get(id);
        if (ticket === undefined) return { ok: false, error: `Ticket not found: ${id}` };
        const updated = await updateTicketStatus(ticket, status);
        return { ok: true, ticket: updated };
      } catch (error: unknown) {
        return { ok: false, error: extractMessage(error) };
      }
    },
    comment: async ({ id, text, section }) => {
      try {
        const { index } = await getIndex();
        const ticket = index.get(id);
        if (ticket === undefined) return { ok: false, error: `Ticket not found: ${id}` };
        await addTicketComment(ticket, text, section);
        return { ok: true };
      } catch (error: unknown) {
        return { ok: false, error: extractMessage(error) };
      }
    },
  };
}
