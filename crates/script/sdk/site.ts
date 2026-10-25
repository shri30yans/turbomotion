// Where in the author's script something was created, for errors found later (a tween merged at write-out, a layer the
// engine rejects). `sdk check` turns tracking on; otherwise nothing is recorded.

/** The first stack frame outside the SDK: the line of the author's script that made this call. */
export function callSite(): string {
  const frames = String(new Error().stack ?? "").split("\n").slice(1);
  return (frames.find((f) => !f.includes("sdk:")) ?? "").trim().replace(/^at\s+/, "");
}

/** Remember the line that created layer `id` (only while the host asked for it). */
export function noteSite(id: string): void {
  const sites = (globalThis as { __tm_sites?: Record<string, string> }).__tm_sites;
  if (sites && sites[id] === undefined) sites[id] = callSite();
}
