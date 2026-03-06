import fs from "node:fs/promises";
import path from "node:path";
import { parseTicketFile, type Ticket } from "./parser.ts";

export type ScanResult = {
  tickets: Ticket[];
  errors: { filePath: string; message: string }[];
};

export async function scanTickets(directoryPath: string): Promise<ScanResult> {
  const tickets: Ticket[] = [];
  const errors: { filePath: string; message: string }[] = [];

  try {
    const entries = await fs.readdir(directoryPath, { withFileTypes: true });
    const sortedEntries = entries
      .filter(
        (e) =>
          e.isFile() &&
          (e.name.endsWith(".md") || e.name.endsWith(".markdown")),
      )
      .sort((a, b) => a.name.localeCompare(b.name, "en"));

    for (const entry of sortedEntries) {
      const filePath = path.join(directoryPath, entry.name);
      try {
        const ticket = await parseTicketFile(filePath);
        tickets.push(ticket);
      } catch (error: any) {
        errors.push({ filePath, message: error.message || String(error) });
      }
    }
  } catch (error: any) {
    if (error.code !== "ENOENT") {
      throw error;
    }
  }

  return { tickets, errors };
}
