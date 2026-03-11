import assert from "node:assert/strict";
import test from "node:test";

import { createAgent, normalizeServerUrl } from "../src/tools/agent.ts";

test("normalizeServerUrl rewrites loopback hosts in container mode", () => {
  assert.equal(
    normalizeServerUrl("http://127.0.0.1:4096", "container"),
    "http://host.docker.internal:4096"
  );
  assert.equal(
    normalizeServerUrl("http://localhost:4096", "container"),
    "http://host.docker.internal:4096"
  );
});

test("normalizeServerUrl keeps host urls in host mode", () => {
  assert.equal(
    normalizeServerUrl("http://127.0.0.1:4096", "host"),
    "http://127.0.0.1:4096"
  );
});

test("agent.run emits session start and preserves compatibility result shape", async () => {
  const events: { name: string; payload: Record<string, unknown> }[] = [];
  const fakeClient = {
    session: {
      async create() {
        return { id: "sess_abc" };
      },
      async prompt() {
        return {
          id: "msg_1",
          parts: [{ type: "text", text: "done" }],
        };
      },
      async messages() {
        return [];
      },
      async abort() {
        return true;
      },
    },
    event: {
      async subscribe() {
        return [];
      },
    },
  };

  const controller = new AbortController();

  const agent = createAgent({
    yolo: false,
    runId: 101,
    runtimeEnv: "host",
    workspaceRoot: "/tmp/project",
    signal: controller.signal,
    emitEvent(name, payload) {
      events.push({ name, payload });
    },
    createClient() {
      return fakeClient;
    },
    async loadSettings() {
      return { opencode: { server_url: "http://127.0.0.1:4096" } };
    },
  });

  const result = await agent.run({ prompt: "hello" });

  assert.equal(result.ok, true);
  assert.equal(result.output, "done");
  assert.equal(result.session_id, "sess_abc");
  assert.equal(result.message_id, "msg_1");
  assert.deepEqual(events, [
    {
      name: "agent_session_started",
      payload: { run_id: 101, session_id: "sess_abc" },
    },
  ]);
});

test("agent messages/events/cancel delegate to sdk client", async () => {
  const calls: string[] = [];
  const fakeClient = {
    session: {
      async create() {
        return { id: "sess_unused" };
      },
      async prompt() {
        return { id: "msg_unused" };
      },
      async messages(input: { path: { id: string } }) {
        calls.push(`messages:${input.path.id}`);
        return [{ id: "m1" }];
      },
      async abort(input: { path: { id: string } }) {
        calls.push(`abort:${input.path.id}`);
        return true;
      },
    },
    event: {
      async subscribe() {
        calls.push("events");
        return [{ properties: { sessionID: "sess_1" } }, "event"];
      },
    },
  };

  const controller = new AbortController();

  const agent = createAgent({
    yolo: false,
    runId: 202,
    runtimeEnv: "host",
    workspaceRoot: "/tmp/project",
    signal: controller.signal,
    emitEvent(_name, _payload) {
      return undefined;
    },
    createClient() {
      return fakeClient;
    },
    async loadSettings() {
      return { opencode: { server_url: "http://127.0.0.1:4096" } };
    },
  });

  const messages = await agent.messages("sess_1");
  const events = await agent.events("sess_1");
  const cancel = await agent.cancel("sess_1");

  assert.deepEqual(messages, {
    session_id: "sess_1",
    messages: [{ id: "m1" }],
  });
  assert.deepEqual(events, {
    session_id: "sess_1",
    stream: [{ properties: { sessionID: "sess_1" } }],
  });
  assert.deepEqual(cancel, {
    session_id: "sess_1",
    result: true,
  });
  assert.deepEqual(calls, ["messages:sess_1", "events", "abort:sess_1"]);
});
