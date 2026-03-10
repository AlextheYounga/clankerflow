import fs from "node:fs/promises";

import matter from "gray-matter";

import {
  normalizeTicketStatus,
  resolveTicketId,
  type TicketStatus,
} from "./schema.ts";

export interface Ticket {
  ticketId: string;
  title: string;
  status: TicketStatus;
  worktree: string;
  description: string | null;
  filePath: string;
  frontmatter: Record<string, unknown>;
}

export function parseTicketContent(content: string, filePath: string): Ticket {
  const { data, content: body } = matter(content);
  const frontmatter = data as Record<string, unknown>;

  const ticketId = resolveTicketId(frontmatter);
  const rawTitle = frontmatter.title;
  const title = typeof rawTitle === "string" ? rawTitle.trim() : "";

  if (ticketId === null || title.length === 0) {
    throw new Error(
      `Ticket missing required fields (id, title) in ${filePath}`
    );
  }

  const rawWorktree = frontmatter.worktree;
  const worktree =
    typeof rawWorktree === "string" ? rawWorktree.trim() : "none";
  const rawStatus =
    typeof frontmatter.status === "string" ? frontmatter.status : undefined;

  return {
    ticketId,
    title,
    status: normalizeTicketStatus(rawStatus),
    worktree,
    description: body.trim().length > 0 ? body.trim() : null,
    filePath,
    frontmatter,
  };
}

export async function parseTicketFile(filePath: string): Promise<Ticket> {
  const content = await fs.readFile(filePath, "utf8");
  return parseTicketContent(content, filePath);
}

export function renderTicketDocument(
  frontmatter: Record<string, unknown>,
  body: string
): string {
  return matter.stringify(
    body.trim().length > 0 ? `\n${body.trim()}\n` : "",
    frontmatter
  );
}
