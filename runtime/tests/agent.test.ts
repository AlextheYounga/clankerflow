import assert from "node:assert/strict";
import test from "node:test";

import type { OpencodeClient } from "@opencode-ai/sdk";

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
      create() {
        return Promise.resolve({ id: "sess_abc" });
      },
      prompt() {
        return Promise.resolve({
          id: "msg_1",
          parts: [{ type: "text", text: "done" }],
        });
      },
      messages() {
        return Promise.resolve([]);
      },
      abort() {
        return Promise.resolve(true);
      },
    },
    event: {
      subscribe() {
        return Promise.resolve([]);
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
      return fakeClient as unknown as OpencodeClient;
    },
    loadSettings() {
      return Promise.resolve({
        opencode: { server_url: "http://127.0.0.1:4096" },
      });
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

test("agent.run returns ok:false with cause detail when session.create fetch fails", async () => {
  const fetchError = new TypeError("fetch failed", {
    cause: new Error("connect ECONNREFUSED 127.0.0.1:4096"),
  });

  const fakeClient = {
    session: {
      create() {
        return Promise.reject(fetchError);
      },
      prompt() {
        return Promise.resolve({});
      },
      messages() {
        return Promise.resolve([]);
      },
      abort() {
        return Promise.resolve(true);
      },
    },
    event: {
      subscribe() {
        return Promise.resolve([]);
      },
    },
  };

  const controller = new AbortController();

  const agent = createAgent({
    yolo: false,
    runId: 303,
    runtimeEnv: "host",
    workspaceRoot: "/tmp/project",
    signal: controller.signal,
    emitEvent() {},
    createClient() {
      return fakeClient as unknown as OpencodeClient;
    },
    loadSettings() {
      return Promise.resolve({
        opencode: { server_url: "http://127.0.0.1:4096" },
      });
    },
  });

  const result = await agent.run({ prompt: "hello" });

  assert.equal(result.ok, false);
  const errorStr = result.error as string;
  assert.ok(
    errorStr.includes("ECONNREFUSED"),
    `expected error to include cause detail, got: "${errorStr}"`
  );
});

test("agent messages/events/cancel delegate to sdk client", async () => {
  const calls: string[] = [];
  const fakeClient = {
    session: {
      create() {
        return Promise.resolve({ id: "sess_unused" });
      },
      prompt() {
        return Promise.resolve({ id: "msg_unused" });
      },
      messages(input: { path: { id: string } }) {
        calls.push(`messages:${input.path.id}`);
        return Promise.resolve([{ id: "m1" }]);
      },
      abort(input: { path: { id: string } }) {
        calls.push(`abort:${input.path.id}`);
        return Promise.resolve(true);
      },
    },
    event: {
      subscribe() {
        calls.push("events");
        return Promise.resolve([
          { properties: { sessionID: "sess_1" } },
          "event",
        ]);
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
      return fakeClient as unknown as OpencodeClient;
    },
    loadSettings() {
      return Promise.resolve({
        opencode: { server_url: "http://127.0.0.1:4096" },
      });
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
