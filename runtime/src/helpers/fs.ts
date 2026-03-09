import fs from "node:fs/promises";
import path from "node:path";

export interface FsContext {
  readText: (relativePath: string) => Promise<string>;
  read: (relativePath: string) => Promise<string>; // Alias for compatibility
  writeText: (relativePath: string, contents: string) => Promise<void>;
  exists: (relativePath: string) => Promise<boolean>;
  listDir: (
    relativePath: string,
  ) => Promise<{ name: string; kind: "file" | "dir" }[]>;
}

export function createFsContext(workspaceRoot: string): FsContext {
  const resolveAndValidatePath = (relativePath: string) => {
    const absolutePath = path.resolve(workspaceRoot, relativePath);
    const normalizedRoot = path.resolve(workspaceRoot);
    if (!absolutePath.startsWith(normalizedRoot)) {
      throw new Error(`Path "${relativePath}" escapes workspace root "${normalizedRoot}"`);
    }
    return absolutePath;
  };

  return {
    readText: async (relativePath: string) =>
      fs.readFile(resolveAndValidatePath(relativePath), "utf8"),
    read: async (relativePath: string) =>
      fs.readFile(resolveAndValidatePath(relativePath), "utf8"),
    writeText: async (relativePath: string, contents: string) =>
      fs.writeFile(resolveAndValidatePath(relativePath), contents),
    exists: async (relativePath: string) => {
      try {
        await fs.access(resolveAndValidatePath(relativePath));
        return true;
      } catch {
        return false;
      }
    },
    listDir: async (relativePath: string) => {
      const absolutePath = resolveAndValidatePath(relativePath);
      const entries = await fs.readdir(absolutePath, { withFileTypes: true });
      return entries.map((entry) => ({
        name: entry.name,
        kind: entry.isDirectory() ? "dir" : "file",
      }));
    },
  };
}
