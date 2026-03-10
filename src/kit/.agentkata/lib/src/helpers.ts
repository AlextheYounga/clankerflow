import { createFsContext } from "./helpers/fs.ts";
import { createGitContext } from "./helpers/git.ts";
import { createTicketContext } from "./helpers/tickets.ts";

const workspaceRoot = process.cwd();

export const fs = createFsContext(workspaceRoot);
export const git = createGitContext(workspaceRoot);
export const tickets = createTicketContext(workspaceRoot);
