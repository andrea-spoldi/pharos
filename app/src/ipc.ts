import type { CommandError } from "./bindings";

type Result<T> = { status: "ok"; data: T } | { status: "error"; error: CommandError };

/** A command's typed error, thrown so screens can use try/catch and resource errors. */
export class IpcError extends Error {
  constructor(readonly error: CommandError) {
    super(error.message);
    this.name = "IpcError";
  }

  get kind() {
    return this.error.kind;
  }
}

export async function unwrap<T>(result: Promise<Result<T>>): Promise<T> {
  const r = await result;
  if (r.status === "error") throw new IpcError(r.error);
  return r.data;
}

export function toIpcError(err: unknown): IpcError {
  return err instanceof IpcError ? err : new IpcError({ kind: "internal", message: String(err) });
}
