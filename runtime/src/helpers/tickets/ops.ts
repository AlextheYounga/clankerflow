import fs from "node:fs/promises";
import {
  parseTicketFile,
  renderTicketDocument,
  type Ticket,
} from "./parser.ts";
import { normalizeTicketStatus, type TicketStatus } from "./schema.ts";

export async function updateTicketStatus(
  ticket: Ticket,
  status: string,
): Promise<Ticket> {
  const nextStatus = normalizeTicketStatus(status);
  const content = await fs.readFile(ticket.filePath, "utf8");
  const { data: frontmatter, content: body } = (
    await import("gray-matter")
  ).default(content);

  frontmatter.status = nextStatus;
  const rendered = renderTicketDocument(frontmatter, body);
  await fs.writeFile(ticket.filePath, rendered);

  return {
    ...ticket,
    status: nextStatus,
    frontmatter,
  };
}

export async function addTicketComment(
  ticket: Ticket,
  text: string,
  section: string = "Comments",
): Promise<void> {
  const content = await fs.readFile(ticket.filePath, "utf8");
  const { data: frontmatter, content: body } = (
    await import("gray-matter")
  ).default(content);

  const heading = `## ${section}`;
  const entry = `- ${text.trim()}`;

  let newBody = body.trim();
  const start = newBody.indexOf(heading);

  if (start !== -1) {
    const afterStart = newBody.slice(start);
    const nextHeadingIndex = afterStart.slice(heading.length).indexOf("\n## ");

    if (nextHeadingIndex !== -1) {
      const insertAt = start + heading.length + nextHeadingIndex;
      newBody = `${newBody.slice(0, insertAt)}\n${entry}${newBody.slice(insertAt)}`;
    } else {
      newBody = `${newBody.trimEnd()}\n${entry}\n`;
    }
  } else {
    newBody = `${newBody.trimEnd()}\n\n${heading}\n\n${entry}\n`;
  }

  const rendered = renderTicketDocument(frontmatter, newBody);
  await fs.writeFile(ticket.filePath, rendered);
}
