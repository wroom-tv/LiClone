export function bytes(n: number | null | undefined): string {
  if (n == null || Number.isNaN(n) || n < 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  const digits = i === 0 ? 0 : v >= 10 ? 1 : 2;
  return `${v.toFixed(digits)} ${units[i]}`;
}

export function rate(n: number | null | undefined): string {
  if (n == null || n <= 0) return "0 B/s";
  return `${bytes(n)}/s`;
}

export function eta(secs: number | null | undefined): string {
  if (secs == null || secs < 0) return "—";
  if (secs < 60) return `${Math.round(secs)}s`;
  const m = Math.floor(secs / 60);
  const s = Math.round(secs % 60);
  if (m < 60) return `${m}m ${s}s`;
  const h = Math.floor(m / 60);
  return `${h}h ${m % 60}m`;
}

export function ago(unix: number): string {
  const delta = Math.max(0, Date.now() / 1000 - unix);
  if (delta < 60) return "just now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
  return `${Math.floor(delta / 86400)}d ago`;
}

export function duration(secs: number | null | undefined): string {
  if (secs == null) return "—";
  return eta(secs);
}

export function pct(n: number | null | undefined): number {
  if (n == null || Number.isNaN(n)) return 0;
  return Math.max(0, Math.min(100, n));
}
