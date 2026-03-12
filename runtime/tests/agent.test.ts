import assert from "node:assert/strict";
import test from "node:test";

import { createAgent } from "../src/tools/agent.ts";

test("agent.run emits session start and preserves compatibility result shape", async () => {
  const events: { name: string; payload: Record<string, unknown> }[] = [];
  const calls: { name: string; payload: Record<string, unknown> }[] = [];

  const agent = createAgent({
    yolo: false,
    runId: 101,
    signal: new AbortController().signal,
    emitEvent(name, payload) {
      events.push({ name, payload });
    },
    invokeCapability(name, payload) {
      calls.push({ name, payload });
      return Promise.resolve({
        output: "done",
        session_id: "sess_abc",
        message_id: "msg_1",
      });
    },
  });

  const result = await agent.run({ prompt: "hello" });

  assert.equal(result.ok, true);
  assert.equal(result.output, "done");
  assert.equal(result.session_id, "sess_abc");
  assert.equal(result.message_id, "msg_1");
  assert.deepEqual(calls, [
    {
      name: "opencode_run",
      payload: { prompt: "hello", yolo: false },
    },
  ]);
  assert.deepEqual(events, [
    {
      name: "agent_session_started",
      payload: { run_id: 101, session_id: "sess_abc" },
    },
  ]);
});

test("agent.run returns ok:false with error message on failure", async () => {
  const agent = createAgent({
    yolo: false,
    runId: 303,
    signal: new AbortController().signal,
    emitEvent() {
      return undefined;
    },
    invokeCapability() {
      return Promise.reject(new Error("server exploded"));
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
  const agent = createAgent({
    yolo: false,
    runId: 404,
    signal: new AbortController().signal,
    emitEvent() {
      return undefined;
    },
    invokeCapability() {
      return Promise.reject(
        new TypeError("request failed", {
          cause: new Error("connect ECONNREFUSED 127.0.0.1:9999"),
        })
      );
    },
  });

  const result = await agent.run({ prompt: "hello" });

  assert.equal(result.ok, false);
  const errorStr = result.error as string;
  assert.ok(
    errorStr.includes("request failed"),
    `expected 'request failed' in error, got: "${errorStr}"`
  );
  assert.ok(
    errorStr.includes("ECONNREFUSED"),
    `expected cause detail in error, got: "${errorStr}"`
  );
});

test("agent command/messages/events/cancel invoke backend capabilities", async () => {
  const calls: string[] = [];
  const agent = createAgent({
    yolo: false,
    runId: 202,
    signal: new AbortController().signal,
    emitEvent() {
      return undefined;
    },
    invokeCapability(name, payload) {
      calls.push(`${name}:${String(payload.session_id ?? "")}`);
      switch (name) {
        case "opencode_messages": {
          return Promise.resolve({
            session_id: payload.session_id,
            messages: [{ id: "m1" }],
          });
        }
        case "opencode_command": {
          return Promise.resolve({
            session_id: payload.session_id,
            response: { status: "executed" },
          });
        }
        case "opencode_events": {
          return Promise.resolve({
            session_id: payload.session_id,
            stream: [{ type: "session.idle" }],
          });
        }
        case "opencode_cancel": {
          return Promise.resolve({
            session_id: payload.session_id,
            result: true,
          });
        }
        default: {
          throw new Error(`unexpected capability: ${name}`);
        }
      }
    },
  });

  const command = await agent.command({
    session_id: "sess_1",
    command: "/review",
  });
  const messages = await agent.messages("sess_1");
  const events = await agent.events("sess_1");
  const cancel = await agent.cancel("sess_1");

  assert.deepEqual(command, {
    session_id: "sess_1",
    response: { status: "executed" },
  });
  assert.deepEqual(messages, {
    session_id: "sess_1",
    messages: [{ id: "m1" }],
  });
  assert.deepEqual(events, {
    session_id: "sess_1",
    stream: [{ type: "session.idle" }],
  });
  assert.deepEqual(cancel, {
    session_id: "sess_1",
    result: true,
  });
  assert.deepEqual(calls, [
    "opencode_command:sess_1",
    "opencode_messages:sess_1",
    "opencode_events:sess_1",
    "opencode_cancel:sess_1",
  ]);
});
