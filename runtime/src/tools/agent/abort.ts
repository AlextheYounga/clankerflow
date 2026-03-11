export async function abortable<T>(
  signal: AbortSignal,
  operation: () => Promise<T>
): Promise<T> {
  if (signal.aborted) {
    throw new Error("operation cancelled");
  }

  return new Promise<T>((resolve, reject) => {
    const onAbort = () => {
      reject(new Error("operation cancelled"));
    };

    signal.addEventListener("abort", onAbort, { once: true });

    operation()
      .then(resolve)
      .catch(reject)
      .finally(() => {
        signal.removeEventListener("abort", onAbort);
      });
  });
}
