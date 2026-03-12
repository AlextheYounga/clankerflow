import assert from "node:assert/strict";
import test from "node:test";

import type { OpencodeClient } from "@opencode-ai/sdk";

import { createAgent, normalizeServerUrl } from "../src/tools/agent.ts";

const FAKE_SERVER_URL = () => Promise.resolve("http://127.0.0.1:4096");

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
        return Promise.resolve({ data: { id: "sess_abc" } });
      },
      prompt() {
        return Promise.resolve({
          data: {
            info: { id: "msg_1" },
            parts: [{ type: "text", text: "done" }],
          },
        });
      },
      messages() {
        return Promise.resolve({ data: [] });
      },
      abort() {
        return Promise.resolve({ data: true });
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
    serverUrl: FAKE_SERVER_URL,
    signal: controller.signal,
    emitEvent(name, payload) {
      events.push({ name, payload });
    },
    createClient() {
      return fakeClient as unknown as OpencodeClient;
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

test("agent.run returns ok:false with error message on failure", async () => {
  const fakeClient = {
    session: {
      create() {
        return Promise.reject(new Error("server exploded"));
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
    serverUrl: FAKE_SERVER_URL,
    signal: controller.signal,
    emitEvent() {
      // noop — this test does not inspect events
    },
    createClient() {
      return fakeClient as unknown as OpencodeClient;
    },
  });

  const result = await agent.run({ prompt: "hello" });

  assert.equal(result.ok, false);
  const errorStr = result.error as string;
  assert.ok(
    errorStr.includes("server exploded"),
    `expected error message, got: "${errorStr}"`
  );
});

test("agent.run chains error.cause into error message", async () => {
  const fetchError = new TypeError("fetch failed", {
    cause: new Error("connect ECONNREFUSED 127.0.0.1:9999"),
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
    runId: 404,
    runtimeEnv: "host",
    workspaceRoot: "/tmp/project",
    serverUrl: () => Promise.resolve("http://127.0.0.1:9999"),
    signal: controller.signal,
    emitEvent() {
      // noop — this test does not inspect events
    },
    createClient() {
      return fakeClient as unknown as OpencodeClient;
    },
  });

  const result = await agent.run({ prompt: "hello" });

  assert.equal(result.ok, false);
  const errorStr = result.error as string;
  assert.ok(
    errorStr.includes("fetch failed"),
    `expected 'fetch failed' in error, got: "${errorStr}"`
  );
  assert.ok(
    errorStr.includes("ECONNREFUSED"),
    `expected cause detail in error, got: "${errorStr}"`
  );
});

test("agent messages/events/cancel delegate to sdk client", async () => {
  const calls: string[] = [];
  const fakeClient = {
    session: {
      create() {
        return Promise.resolve({ data: { id: "sess_unused" } });
      },
      prompt() {
        return Promise.resolve({
          data: {
            info: { id: "msg_unused" },
            parts: [],
          },
        });
      },
      messages(input: { path: { id: string } }) {
        calls.push(`messages:${input.path.id}`);
        return Promise.resolve({ data: [{ id: "m1" }] });
      },
      abort(input: { path: { id: string } }) {
        calls.push(`abort:${input.path.id}`);
        return Promise.resolve({ data: true });
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
    serverUrl: FAKE_SERVER_URL,
    signal: controller.signal,
    emitEvent(_name, _payload) {
      return undefined;
    },
    createClient() {
      return fakeClient as unknown as OpencodeClient;
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
