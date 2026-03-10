import { simpleGit, type SimpleGit, type SimpleGitOptions } from "simple-git";

export interface GitResult {
  ok: boolean;
  code: number;
  stdout: string;
  stderr: string;
  command: string;
}

export interface GitContext {
  status: () => Promise<GitResult>;
  diff: () => Promise<GitResult>;
  add: (files: string | string[]) => Promise<GitResult>;
  commit: (message: string) => Promise<GitResult>;
  push: (remote?: string, branch?: string) => Promise<GitResult>;
  pull: (remote?: string, branch?: string) => Promise<GitResult>;
  log: (options?: string[]) => Promise<GitResult>;
  checkout: (branch: string) => Promise<GitResult>;
  checkoutBranch: (branch: string, startPoint: string) => Promise<GitResult>;
}

function errorCode(error: unknown): number {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof (error as { code: unknown }).code === "number"
  ) {
    return (error as { code: number }).code;
  }
  return 1;
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

async function wrap<T>(
  git: SimpleGit,
  cmd: string,
  fn: (g: SimpleGit) => Promise<T>
): Promise<GitResult> {
  try {
    const result = await fn(git);
    return {
      ok: true,
      code: 0,
      stdout: typeof result === "string" ? result : JSON.stringify(result),
      stderr: "",
      command: cmd,
    };
  } catch (error: unknown) {
    return {
      ok: false,
      code: errorCode(error),
      stdout: "",
      stderr: errorMessage(error),
      command: cmd,
    };
  }
}

function filesLabel(files: string | string[]): string {
  return Array.isArray(files) ? files.join(" ") : files;
}

export function createGitContext(workspaceRoot: string): GitContext {
  const options: Partial<SimpleGitOptions> = {
    baseDir: workspaceRoot,
    binary: "git",
    maxConcurrentProcesses: 6,
    trimmed: true,
  };

  const git: SimpleGit = simpleGit(options);

  return {
    status: () => wrap(git, "git status", (g) => g.status()),
    diff: () => wrap(git, "git diff", (g) => g.diff()),
    add: (files: string | string[]) =>
      wrap(git, `git add ${filesLabel(files)}`, (g) => g.add(files)),
    commit: (message: string) =>
      wrap(git, `git commit -m "${message}"`, (g) => g.commit(message)),
    push: (remote?: string, branch?: string) =>
      wrap(git, `git push ${remote ?? ""} ${branch ?? ""}`, (g) =>
        g.push(remote, branch)
      ),
    pull: (remote?: string, branch?: string) =>
      wrap(git, `git pull ${remote ?? ""} ${branch ?? ""}`, (g) =>
        g.pull(remote, branch)
      ),
    log: (options?: string[]) => wrap(git, "git log", (g) => g.log(options)),
    checkout: (branch: string) =>
      wrap(git, `git checkout ${branch}`, (g) => g.checkout(branch)),
    checkoutBranch: (branch: string, startPoint: string) =>
      wrap(git, `git checkout -b ${branch} ${startPoint}`, (g) =>
        g.checkoutBranch(branch, startPoint)
      ),
  };
}
