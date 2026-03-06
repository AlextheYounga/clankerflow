import { simpleGit, type SimpleGit, type SimpleGitOptions } from "simple-git";

export type GitResult = {
  ok: boolean;
  code: number;
  stdout: string;
  stderr: string;
  command: string;
};

export type GitContext = {
  status: () => Promise<GitResult>;
  diff: () => Promise<GitResult>;
  add: (files: string | string[]) => Promise<GitResult>;
  commit: (message: string) => Promise<GitResult>;
  push: (remote?: string, branch?: string) => Promise<GitResult>;
  pull: (remote?: string, branch?: string) => Promise<GitResult>;
  log: (options?: string[]) => Promise<GitResult>;
  checkout: (branch: string) => Promise<GitResult>;
  checkoutBranch: (branch: string, startPoint: string) => Promise<GitResult>;
};

export function createGitContext(workspaceRoot: string): GitContext {
  const options: Partial<SimpleGitOptions> = {
    baseDir: workspaceRoot,
    binary: "git",
    maxConcurrentProcesses: 6,
    trimmed: true,
  };

  const git: SimpleGit = simpleGit(options);

  async function wrap<T>(
    cmd: string,
    fn: () => Promise<T>,
  ): Promise<GitResult> {
    try {
      const result = await fn();
      return {
        ok: true,
        code: 0,
        stdout: typeof result === "string" ? result : JSON.stringify(result),
        stderr: "",
        command: cmd,
      };
    } catch (error: any) {
      return {
        ok: false,
        code: error.code || 1,
        stdout: "",
        stderr: error.message || String(error),
        command: cmd,
      };
    }
  }

  return {
    status: () => wrap("git status", () => git.status()),
    diff: () => wrap("git diff", () => git.diff()),
    add: (files: string | string[]) =>
      wrap(`git add ${files}`, () => git.add(files)),
    commit: (message: string) =>
      wrap(`git commit -m "${message}"`, () => git.commit(message)),
    push: (remote?: string, branch?: string) =>
      wrap(`git push ${remote || ""} ${branch || ""}`, () =>
        git.push(remote, branch),
      ),
    pull: (remote?: string, branch?: string) =>
      wrap(`git pull ${remote || ""} ${branch || ""}`, () =>
        git.pull(remote, branch),
      ),
    log: (options?: string[]) => wrap("git log", () => git.log(options)),
    checkout: (branch: string) =>
      wrap(`git checkout ${branch}`, () => git.checkout(branch)),
    checkoutBranch: (branch: string, startPoint: string) =>
      wrap(`git checkout -b ${branch} ${startPoint}`, () =>
        git.checkoutBranch(branch, startPoint),
      ),
  };
}
