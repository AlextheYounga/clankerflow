export type EventEmitter = (
  name: string,
  payload: Record<string, unknown>
) => void;

export interface LogContext {
  debug(message: string): void;
  info(message: string): void;
  warn(message: string): void;
  error(message: string): void;
}

export function createLogContext(emit: EventEmitter): LogContext {
  const log = (level: string, message: string) =>
    emit("log", { level, target: "workflow", message });
  return {
    debug: (message: string) => log("debug", message),
    info: (message: string) => log("info", message),
    warn: (message: string) => log("warn", message),
    error: (message: string) => log("error", message),
  };
}
