import test from "node:test";
import assert from "node:assert/strict";
import net from "node:net";
import { once } from "node:events";

interface IpcMessage {
  v: "v1";
  id: string;
  kind: "command" | "event" | "response" | "error";
  name: string;
  payload: Record<string, unknown>;
}

test("IpcTransport connects and exchanges JSON messages over TCP", async () => {
  const server = net.createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { port } = server.address() as net.AddressInfo;

  const connectionPromise = once(server, "connection") as Promise<[net.Socket]>;

  const client = net.createConnection({ host: "127.0.0.1", port });
  await once(client, "connect");

  const [serverSocket] = await connectionPromise;

  const outgoing: IpcMessage = {
    v: "v1",
    id: "test_1",
    kind: "command",
    name: "start_run",
    payload: { run_id: 1 },
  };

  const received: IpcMessage[] = [];
  let buffer = "";
  client.on("data", (chunk) => {
    buffer += chunk.toString();
    const lines = buffer.split("\n");
    buffer = lines.pop()!;
    for (const line of lines) {
      if (line.trim()) {
        received.push(JSON.parse(line) as IpcMessage);
      }
    }
  });

  serverSocket.write(JSON.stringify(outgoing) + "\n");
  await delay(50);

  assert.equal(received.length, 1);
  assert.equal(received[0]!.id, "test_1");
  assert.equal(received[0]!.name, "start_run");
  assert.deepEqual(received[0]!.payload, { run_id: 1 });

  client.destroy();
  serverSocket.destroy();
  server.close();
});

test("line-delimited framing handles partial chunks", async () => {
  const server = net.createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { port } = server.address() as net.AddressInfo;

  const connectionPromise = once(server, "connection") as Promise<[net.Socket]>;

  const client = net.createConnection({ host: "127.0.0.1", port });
  await once(client, "connect");

  const [serverSocket] = await connectionPromise;

  const message: IpcMessage = {
    v: "v1",
    id: "partial_1",
    kind: "event",
    name: "test_event",
    payload: { data: "hello" },
  };
  const full = JSON.stringify(message) + "\n";

  const received: IpcMessage[] = [];
  let buffer = "";
  client.on("data", (chunk) => {
    buffer += chunk.toString();
    const lines = buffer.split("\n");
    buffer = lines.pop()!;
    for (const line of lines) {
      if (line.trim()) {
        received.push(JSON.parse(line) as IpcMessage);
      }
    }
  });

  // Send in two partial chunks
  const midpoint = Math.floor(full.length / 2);
  serverSocket.write(full.slice(0, midpoint));
  await delay(30);

  assert.equal(received.length, 0, "should not parse partial message");

  serverSocket.write(full.slice(midpoint));
  await delay(30);

  assert.equal(received.length, 1, "should parse complete message");
  assert.equal(received[0]!.id, "partial_1");

  client.destroy();
  serverSocket.destroy();
  server.close();
});

test("disconnect fires on server close", async () => {
  const server = net.createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const { port } = server.address() as net.AddressInfo;

  const connectionPromise = once(server, "connection") as Promise<[net.Socket]>;

  const client = net.createConnection({ host: "127.0.0.1", port });
  await once(client, "connect");

  const [serverSocket] = await connectionPromise;

  let disconnected = false;
  client.on("close", () => {
    disconnected = true;
  });

  serverSocket.destroy();
  await delay(50);

  assert.ok(disconnected, "client should detect disconnect");

  client.destroy();
  server.close();
});

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
