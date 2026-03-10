// Public type declarations for agentkata workflow authoring.
//
// Usage:
//   import type { WorkflowMeta, WorkflowContext } from "agentkata";

export interface WorkflowMeta {
  id: string;
  name: string;
  description?: string;
  runtime: "host" | "container";
}

export interface AgentRunInput {
  title: string;
  prompt: string;
  yolo?: boolean;
  [key: string]: unknown;
}

export interface AgentRunResult {
  ok: boolean;
  output?: string;
  error?: string;
  session_id?: string;
  message_id?: string;
  [key: string]: unknown;
}

export interface AgentContext {
  run(input: AgentRunInput): Promise<AgentRunResult>;
  events(sessionId: string): Promise<Record<string, unknown>>;
  messages(sessionId: string): Promise<Record<string, unknown>>;
  cancel(sessionId: string): Promise<Record<string, unknown>>;
}

export interface ExecResult {
  code: number;
  stdout: string;
  stderr: string;
}

export interface LogContext {
  debug(message: string): void;
  info(message: string): void;
  warn(message: string): void;
  error(message: string): void;
}

export type TicketStatus =
  | "OPEN"
  | "IN_PROGRESS"
  | "QA_REVIEW"
  | "QA_CHANGES_REQUESTED"
  | "STUCK"
  | "CLOSED";

export interface Ticket {
  ticketId: string;
  title: string;
  status: TicketStatus;
  worktree: string;
  description: string | null;
  filePath: string;
  frontmatter: Record<string, unknown>;
}

export interface WorkflowContext {
  yolo: boolean;
  ticket: Ticket | null;
  agent: AgentContext;
  exec(command: string, args?: string[]): Promise<ExecResult>;
  log: LogContext;
  sleep(ms: number): Promise<void>;
  signal: AbortSignal;
}

export interface FsContext {
  readText(relativePath: string): Promise<string>;
  read(relativePath: string): Promise<string>;
  writeText(relativePath: string, contents: string): Promise<void>;
  exists(relativePath: string): Promise<boolean>;
  listDir(
    relativePath: string,
  ): Promise<{ name: string; kind: "file" | "dir" }[]>;
}

export interface GitResult {
  ok: boolean;
  code: number;
  stdout: string;
  stderr: string;
  command: string;
}

export interface GitContext {
  status(): Promise<GitResult>;
  diff(): Promise<GitResult>;
  add(files: string | string[]): Promise<GitResult>;
  commit(message: string): Promise<GitResult>;
  push(remote?: string, branch?: string): Promise<GitResult>;
  pull(remote?: string, branch?: string): Promise<GitResult>;
  log(options?: string[]): Promise<GitResult>;
  checkout(branch: string): Promise<GitResult>;
  checkoutBranch(branch: string, startPoint: string): Promise<GitResult>;
}

export interface TicketContext {
  list(): Promise<{ ok: boolean; tickets: Ticket[]; errors: unknown[] }>;
  get(options: {
    id: string;
  }): Promise<{ ok: boolean; ticket?: Ticket; error?: string }>;
  getNext(options?: {
    status?: string;
  }): Promise<{ ok: boolean; ticket?: Ticket; error?: string }>;
  updateStatus(options: {
    id: string;
    status: string;
  }): Promise<{ ok: boolean; ticket?: Ticket; error?: string }>;
  comment(options: {
    id: string;
    text: string;
    section?: string;
  }): Promise<{ ok: boolean; error?: string }>;
}
