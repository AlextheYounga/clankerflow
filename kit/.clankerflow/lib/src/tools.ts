import { createFsContext } from "./tools/fs.ts";
import { createGitContext } from "./tools/git.ts";
import { createTicketContext } from "./tools/tickets.ts";

const workspaceRoot = process.cwd();

export const fs = createFsContext(workspaceRoot);
export const git = createGitContext(workspaceRoot);
export const tickets = createTicketContext(workspaceRoot);
