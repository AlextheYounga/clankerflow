import path from "node:path";
import { scanTickets } from "./scanner.ts";
import { TicketLookup } from "./lookup.ts";
import { type Ticket } from "./parser.ts";
import { normalizeTicketStatus, TicketStatus } from "./schema.ts";
import { updateTicketStatus, addTicketComment } from "./ops.ts";

export type TicketContext = {
  list: () => Promise<{ ok: boolean; tickets: Ticket[]; errors: any[] }>;
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
};

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
      } catch (error: any) {
        return {
          ok: false,
          tickets: [],
          errors: [error.message],
          error: error.message,
        };
      }
    },
    get: async ({ id }) => {
      try {
        const { index } = await getIndex();
        const ticket = index.get(id);
        if (!ticket) return { ok: false, error: `Ticket not found: ${id}` };
        return { ok: true, ticket };
      } catch (error: any) {
        return { ok: false, error: error.message };
      }
    },
    getNext: async (options) => {
      try {
        const status = normalizeTicketStatus(options?.status || "OPEN");
        const { index } = await getIndex();
        const ticket = index.getNextByStatus(status);
        return { ok: true, ticket };
      } catch (error: any) {
        return { ok: false, error: error.message };
      }
    },
    updateStatus: async ({ id, status }) => {
      try {
        const { index } = await getIndex();
        const ticket = index.get(id);
        if (!ticket) return { ok: false, error: `Ticket not found: ${id}` };
        const updated = await updateTicketStatus(ticket, status);
        return { ok: true, ticket: updated };
      } catch (error: any) {
        return { ok: false, error: error.message };
      }
    },
    comment: async ({ id, text, section }) => {
      try {
        const { index } = await getIndex();
        const ticket = index.get(id);
        if (!ticket) return { ok: false, error: `Ticket not found: ${id}` };
        await addTicketComment(ticket, text, section);
        return { ok: true };
      } catch (error: any) {
        return { ok: false, error: error.message };
      }
    },
  };
}
