const pad = (n: number) => String(n).padStart(2, "0");

// Server ISO string -> value for <input type="datetime-local">
export function toLocalInput(iso: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

// Value from <input type="datetime-local"> -> ISO string with timezone for the server
export function fromLocalInput(value: string): string {
  return new Date(value).toISOString();
}

// e.g. "America/New_York", shown next to the deadline inputs
export const browserTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
