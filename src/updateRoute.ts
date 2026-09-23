export type UpdateStep = {
  version: string;
  min: string;
  max?: string;
};

export type UpdateRoute = {
  route: UpdateStep[];
};

export const UPDATE_ROUTE_URL =
  "https://github.com/wroom-tv/LiClone/releases/latest/download/updates.json";

function parts(version: string): number[] {
  return version
    .replace(/^v/i, "")
    .split(".")
    .map((piece) => {
      const match = /^(\d+)/.exec(piece);
      return match ? Number(match[1]) : 0;
    });
}

export function compareVersions(left: string, right: string): number {
  const a = parts(left);
  const b = parts(right);
  const count = Math.max(a.length, b.length);
  for (let i = 0; i < count; i += 1) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

/** The nearest newer release this install is allowed to take. */
export function nextHop(current: string, route: UpdateStep[]): string | null {
  const allowed = route.filter((step) => {
    if (compareVersions(current, step.min) < 0) return false;
    if (compareVersions(step.version, current) <= 0) return false;
    if (step.max && compareVersions(current, step.max) > 0) return false;
    return true;
  });
  allowed.sort((a, b) => compareVersions(a.version, b.version));
  return allowed[0]?.version ?? null;
}

export function isBridge(next: string, route: UpdateStep[]): boolean {
  return route.some((step) => compareVersions(step.version, next) > 0);
}
