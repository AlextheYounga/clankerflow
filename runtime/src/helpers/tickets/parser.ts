import matter from "gray-matter";
import fs from "node:fs/promises";
import {
  normalizeTicketStatus,
  resolveTicketId,
  TicketStatus,
} from "./schema.ts";

export type Ticket = {
  ticketId: string;
  title: string;
  status: TicketStatus;
  worktree: string;
  description: string | null;
  filePath: string;
  frontmatter: Record<string, any>;
};

export function parseTicketContent(content: string, filePath: string): Ticket {
  const { data: frontmatter, content: body } = matter(content);

  const ticketId = resolveTicketId(frontmatter);
  const title = String(frontmatter.title || "").trim();

  if (!ticketId || !title) {
    throw new Error(
      `Ticket missing required fields (id, title) in ${filePath}`,
    );
  }

  return {
    ticketId,
    title,
    status: normalizeTicketStatus(frontmatter.status),
    worktree: String(frontmatter.worktree || "none").trim(),
    description: body.trim() || null,
    filePath,
    frontmatter,
  };
}

export async function parseTicketFile(filePath: string): Promise<Ticket> {
  const content = await fs.readFile(filePath, "utf8");
  return parseTicketContent(content, filePath);
}

export function renderTicketDocument(
  frontmatter: Record<string, any>,
  body: string,
): string {
  return matter.stringify(body.trim() ? `\n${body.trim()}\n` : "", frontmatter);
}
