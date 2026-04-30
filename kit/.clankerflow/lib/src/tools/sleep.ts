export function sleepWithSignal(
  ms: number,
  signal: AbortSignal
): Promise<void> {
  if (signal.aborted) {
    return Promise.reject(new Error("operation cancelled"));
  }

  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      signal.removeEventListener("abort", onAbort);
      resolve();
    }, ms);

    const onAbort = () => {
      clearTimeout(timer);
      reject(new Error("operation cancelled"));
    };
    signal.addEventListener("abort", onAbort, { once: true });
  });
}
