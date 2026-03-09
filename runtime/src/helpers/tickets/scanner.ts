import fs from "node:fs/promises";
import path from "node:path";

import { parseTicketFile, type Ticket } from "./parser.ts";

export interface ScanResult {
  tickets: Ticket[];
  errors: { filePath: string; message: string }[];
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

function isEnoent(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    (error as { code: unknown }).code === "ENOENT"
  );
}

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
      } catch (error: unknown) {
        errors.push({ filePath, message: errorMessage(error) });
      }
    }
  } catch (error: unknown) {
    if (!isEnoent(error)) {
      throw error;
    }
  }

  return { tickets, errors };
}
