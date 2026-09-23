import type { ReactElement } from "react";

type Tab = "instances" | "mounts" | "cache" | "uploads" | "transfers" | "remotes" | "schedule";

const stroke = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.7,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

export function TabIcon({ name }: { name: Tab }): ReactElement {
  if (name === "instances") {
    return (
      <svg viewBox="0 0 24 24" {...stroke}>
        <rect x="3" y="3.5" width="18" height="7" rx="2" />
        <rect x="3" y="13.5" width="18" height="7" rx="2" />
        <path d="M6.5 7h6M6.5 17h4" />
        <circle cx="17" cy="17" r="1.2" fill="currentColor" stroke="none" />
      </svg>
    );
  }
  if (name === "mounts") {
    return (
      <svg viewBox="0 0 24 24" {...stroke}>
        <path d="M4 15.5h16v3.2a1.6 1.6 0 0 1-1.6 1.6H5.6A1.6 1.6 0 0 1 4 18.7v-3.2Z" />
        <path d="M6.2 15.5 8.4 8.2A2 2 0 0 1 10.3 7h3.4a2 2 0 0 1 1.9 1.2l2.2 7.3" />
        <path d="M8 19.2h2.2" />
      </svg>
    );
  }
  if (name === "cache") {
    return (
      <svg viewBox="0 0 24 24" {...stroke}>
        <ellipse cx="12" cy="6" rx="7" ry="2.6" />
        <path d="M5 6v6c0 1.5 3.1 2.6 7 2.6s7-1.1 7-2.6V6" />
        <path d="M5 12v6c0 1.5 3.1 2.6 7 2.6s7-1.1 7-2.6v-6" />
      </svg>
    );
  }
  if (name === "uploads") {
    return (
      <svg viewBox="0 0 24 24" {...stroke}>
        <path d="M12 16V6" />
        <path d="M8.2 9.6 12 5.8l3.8 3.8" />
        <path d="M5 18.5h14" />
      </svg>
    );
  }
  if (name === "transfers") {
    return (
      <svg viewBox="0 0 24 24" {...stroke}>
        <path d="M7 18h10a4 4 0 0 0 .4-8 5 5 0 0 0-9.6-1.4A3.5 3.5 0 0 0 7 18Z" />
        <path d="M12 16.5V9" />
        <path d="M9.2 11.2 12 8.4l2.8 2.8" />
      </svg>
    );
  }
  if (name === "remotes") {
    return (
      <svg viewBox="0 0 24 24" {...stroke}>
        <path d="M7.5 16.5h9.2a3.4 3.4 0 0 0 .3-6.8 4.6 4.6 0 0 0-8.8-1.2 3.2 3.2 0 0 0-.7 8Z" />
        <circle cx="9" cy="15" r="0.9" fill="currentColor" stroke="none" />
        <circle cx="12" cy="15" r="0.9" fill="currentColor" stroke="none" />
        <circle cx="15" cy="15" r="0.9" fill="currentColor" stroke="none" />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 24 24" {...stroke}>
      <rect x="4" y="5" width="16" height="15" rx="2" />
      <path d="M4 9.5h16M8 3.5v3M16 3.5v3" />
      <circle cx="12" cy="15" r="2.4" />
      <path d="M12 14.2V15l.8.6" />
    </svg>
  );
}
