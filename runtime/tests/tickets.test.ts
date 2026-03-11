import { test } from "node:test";
import assert from "node:assert";
import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";

import { createTicketContext } from "../src/tools/tickets.ts";

test("TicketContext: CRUD operations", async (t) => {
  const tmpDir = await fs.mkdtemp(
    path.join(os.tmpdir(), "agentkata-ticket-test-")
  );
  const ticketsDir = path.join(tmpDir, ".agents", "tickets");
  await fs.mkdir(ticketsDir, { recursive: true });

  const ticket1Content = `---
id: '001'
title: Test Ticket 1
status: OPEN
worktree: none
---
Body content 1`;

  const ticket2Content = `---
id: '002'
title: Test Ticket 2
status: IN_PROGRESS
branch: feature/ticket-002
worktree: some-path
---
Body content 2`;

  await fs.writeFile(path.join(ticketsDir, "T-001.md"), ticket1Content);
  await fs.writeFile(path.join(ticketsDir, "T-002.md"), ticket2Content);

  const ticketCtx = createTicketContext(tmpDir);

  await t.test("should list tickets", async () => {
    const result = await ticketCtx.list();
    assert.strictEqual(result.ok, true);
    assert.strictEqual(result.tickets.length, 2);
  });

  await t.test("should get ticket by id", async () => {
    const result = await ticketCtx.get({ id: "001" });
    assert.strictEqual(result.ok, true);
    assert.ok(result.ticket);
    assert.strictEqual(result.ticket.title, "Test Ticket 1");
    assert.strictEqual(result.ticket.branch, null);
  });

  await t.test("should parse branch from ticket frontmatter", async () => {
    const result = await ticketCtx.get({ id: "002" });
    assert.strictEqual(result.ok, true);
    assert.strictEqual(result.ticket?.branch, "feature/ticket-002");
  });

  await t.test(
    "should get next open ticket by id (priority removed)",
    async () => {
      const result = await ticketCtx.getNext({ status: "OPEN" });
      assert.strictEqual(result.ok, true);
      assert.strictEqual(result.ticket?.ticketId, "001");
    }
  );

  await t.test("should update ticket status", async () => {
    const result = await ticketCtx.updateStatus({
      id: "001",
      status: "CLOSED",
    });
    assert.strictEqual(result.ok, true);
    assert.strictEqual(result.ticket?.status, "CLOSED");

    const content = await fs.readFile(
      path.join(ticketsDir, "T-001.md"),
      "utf8"
    );
    assert.ok(content.includes("status: CLOSED"));
  });

  await t.test("should add a comment", async () => {
    const result = await ticketCtx.comment({
      id: "001",
      text: "this is a comment",
    });
    assert.strictEqual(result.ok, true);

    const content = await fs.readFile(
      path.join(ticketsDir, "T-001.md"),
      "utf8"
    );
    assert.ok(content.includes("## Comments"));
    assert.ok(content.includes("- this is a comment"));
  });

  await fs.rm(tmpDir, { recursive: true });
});
