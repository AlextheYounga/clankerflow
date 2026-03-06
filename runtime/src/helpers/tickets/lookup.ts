import { type Ticket } from "./parser.ts";
import { TicketStatus } from "./schema.ts";

export class TicketLookup {
  private byId = new Map<string, Ticket>();
  private byStatus = new Map<TicketStatus, Ticket[]>();

  constructor(tickets: Ticket[]) {
    for (const ticket of tickets) {
      this.byId.set(ticket.ticketId, ticket);
      const bucket = this.byStatus.get(ticket.status) || [];
      bucket.push(ticket);
      this.byStatus.set(ticket.status, bucket);
    }

    for (const [status, bucket] of this.byStatus.entries()) {
      this.byStatus.set(status, this.sortTickets(bucket));
    }
  }

  private sortTickets(tickets: Ticket[]): Ticket[] {
    return [...tickets].sort((a, b) => {
      return a.ticketId.localeCompare(b.ticketId, "en", { numeric: true });
    });
  }

  list(): Ticket[] {
    return Array.from(this.byId.values());
  }

  get(id: string): Ticket | undefined {
    return this.byId.get(id);
  }

  listByStatus(status: TicketStatus): Ticket[] {
    return this.byStatus.get(status) || [];
  }

  getNextByStatus(status: TicketStatus): Ticket | undefined {
    const bucket = this.listByStatus(status);
    return bucket.length > 0 ? bucket[0] : undefined;
  }
}
