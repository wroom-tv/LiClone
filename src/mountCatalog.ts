import { emptyMount, type MountProfile, type RcloneInstance } from "./types";

export type MountSetting = {
  key: string;
  label: string;
  help: string;
  group: string;
  kind: "text" | "choice" | "range" | "menu" | "bool" | "folder";
  placeholder?: string;
  options?: { value: string; label: string }[];
  advanced?: boolean;
};

export const MOUNT_GROUPS = [
  { id: "cache", title: "Cache" },
  { id: "read", title: "Reading" },
  { id: "speed", title: "Speed" },
  { id: "access", title: "Windows and access" },
  { id: "reliability", title: "Reliability" },
  { id: "status", title: "Live status" },
];

export const MOUNT_SETTINGS: MountSetting[] = [
  {
    key: "vfs-cache-mode",
    group: "cache",
    label: "Cache mode",
    kind: "choice",
    options: [
      { value: "off", label: "Off" },
      { value: "minimal", label: "While a file is open" },
      { value: "writes", label: "Until the upload finishes" },
      { value: "full", label: "Reads and writes" },
    ],
    help: "Off streams every read from the cloud. While open only keeps a file for as long as an app has it open. Until the upload finishes keeps your saves on disk until they reach the cloud. Reads and writes also keeps files you have opened, so the next open can be local.",
  },
  {
    key: "cache-dir",
    group: "cache",
    label: "Cache directory",
    kind: "folder",
    placeholder: "rclone default",
    help: "Folder for cached file data and upload metadata. Leave empty to use rclone’s default.",
  },
  {
    key: "vfs-cache-max-size",
    group: "cache",
    label: "Cache size limit",
    kind: "range",
    options: [
      { value: "", label: "No limit" },
      { value: "1G", label: "1 GB" },
      { value: "5G", label: "5 GB" },
      { value: "10G", label: "10 GB" },
      { value: "20G", label: "20 GB" },
      { value: "50G", label: "50 GB" },
      { value: "100G", label: "100 GB" },
    ],
    help: "Disk space rclone may use for cached files. When the cache grows past this, the oldest files that are already uploaded are deleted. Files still uploading are kept.",
  },
  {
    key: "vfs-cache-max-age",
    group: "cache",
    label: "Cache max age",
    kind: "range",
    options: [
      { value: "1h", label: "1 hour" },
      { value: "6h", label: "6 hours" },
      { value: "24h", label: "1 day" },
      { value: "168h", label: "1 week" },
      { value: "720h", label: "1 month" },
      { value: "8760h", label: "Keep a year" },
    ],
    help: "How long a cached file can sit unused before rclone is allowed to delete it. This does not delete a file that still needs to upload.",
  },
  {
    key: "vfs-write-back",
    group: "cache",
    label: "Write-back delay",
    kind: "range",
    options: [
      { value: "0s", label: "Immediately" },
      { value: "1s", label: "1 second" },
      { value: "5s", label: "5 seconds" },
      { value: "15s", label: "15 seconds" },
      { value: "1m", label: "1 minute" },
    ],
    help: "After you save a file, rclone waits this long in case you save again, then starts the upload. Shorter means the cloud copy appears sooner.",
  },
  {
    key: "vfs-cache-poll-interval",
    group: "cache",
    label: "Cache cleanup interval",
    kind: "menu",
    options: [
      { value: "30s", label: "30 seconds" },
      { value: "1m", label: "1 minute" },
      { value: "5m", label: "5 minutes" },
      { value: "15m", label: "15 minutes" },
    ],
    advanced: true,
    help: "How often rclone looks through the cache for files that are old enough or large enough to delete.",
  },
  {
    key: "buffer-size",
    group: "read",
    label: "Memory buffer per file",
    kind: "range",
    options: [
      { value: "8M", label: "8 MB" },
      { value: "16M", label: "16 MB" },
      { value: "32M", label: "32 MB" },
      { value: "64M", label: "64 MB" },
      { value: "128M", label: "128 MB" },
    ],
    help: "Memory used to read ahead for each open file. Higher is smoother for video and uses more RAM.",
  },
  {
    key: "vfs-read-ahead",
    group: "read",
    label: "Read ahead",
    kind: "range",
    options: [
      { value: "0", label: "Off" },
      { value: "32M", label: "32 MB" },
      { value: "128M", label: "128 MB" },
      { value: "256M", label: "256 MB" },
      { value: "1G", label: "1 GB" },
    ],
    help: "How far past the current playback position rclone keeps fetching. Off waits until the app asks for the next piece. Higher reduces stutter and uses more cache.",
  },
  {
    key: "dir-cache-time",
    group: "read",
    label: "Folder listing cache",
    kind: "menu",
    options: [
      { value: "15s", label: "15 seconds" },
      { value: "1m", label: "1 minute" },
      { value: "5m", label: "5 minutes" },
      { value: "15m", label: "15 minutes" },
      { value: "30m", label: "30 minutes" },
      { value: "1h", label: "1 hour" },
      { value: "24h", label: "1 day" },
    ],
    help: "After a folder is listed, that list is reused for this long. Opening the same folder again inside the window is instant. When it ends, the next open asks the cloud for a fresh list.",
  },
  {
    key: "poll-interval",
    group: "read",
    label: "Watch the cloud for changes",
    kind: "menu",
    options: [
      { value: "0", label: "Off" },
      { value: "15s", label: "Every 15 seconds" },
      { value: "1m", label: "Every minute" },
      { value: "5m", label: "Every 5 minutes" },
      { value: "15m", label: "Every 15 minutes" },
      { value: "1h", label: "Every hour" },
    ],
    help: "Separate from the folder listing cache beside it. This timer asks the cloud whether someone else added, changed, or deleted a file, even while you leave the folder open. Off means an open folder stays as it was until you leave it and come back. Shorter notices those outside changes sooner and uses more cloud requests. Files you save on this PC are not waiting on this timer.",
  },
  {
    key: "attr-timeout",
    group: "read",
    label: "Attribute cache",
    kind: "menu",
    options: [
      { value: "1s", label: "1 second" },
      { value: "5s", label: "5 seconds" },
      { value: "30s", label: "30 seconds" },
      { value: "1m", label: "1 minute" },
    ],
    help: "How long Explorer can trust a file’s size and date before asking again. Longer is fewer cloud calls. Shorter picks up a rename or a new size sooner.",
  },
  {
    key: "vfs-read-chunk-size",
    group: "read",
    label: "Read chunk size",
    kind: "menu",
    options: [
      { value: "8M", label: "8 MB" },
      { value: "32M", label: "32 MB" },
      { value: "128M", label: "128 MB" },
      { value: "256M", label: "256 MB" },
    ],
    help: "How much of a file rclone fetches from the cloud in one request. Larger is better for video and big copies.",
  },
  {
    key: "transfers",
    group: "speed",
    label: "Parallel transfers",
    kind: "menu",
    options: [
      { value: "1", label: "1" },
      { value: "2", label: "2" },
      { value: "4", label: "4" },
      { value: "8", label: "8" },
      { value: "16", label: "16" },
    ],
    help: "How many files can upload or download at the same time. Higher finishes a batch sooner and puts more load on the cloud and this PC.",
  },
  {
    key: "checkers",
    group: "speed",
    label: "Parallel checkers",
    kind: "menu",
    options: [
      { value: "2", label: "2" },
      { value: "4", label: "4" },
      { value: "8", label: "8" },
      { value: "16", label: "16" },
      { value: "32", label: "32" },
    ],
    help: "How many files rclone can compare at once when it is checking what changed. Higher is faster and uses more requests.",
  },
  {
    key: "multi-thread-streams",
    group: "speed",
    label: "Streams per large file",
    kind: "menu",
    options: [
      { value: "", label: "Default" },
      { value: "2", label: "2" },
      { value: "4", label: "4" },
      { value: "8", label: "8" },
    ],
    help: "For a large file, how many parallel downloads rclone uses. Default leaves that decision to rclone.",
  },
  {
    key: "bwlimit",
    group: "speed",
    label: "Bandwidth limit",
    kind: "range",
    options: [
      { value: "", label: "Off" },
      { value: "1M", label: "1 MB/s" },
      { value: "5M", label: "5 MB/s" },
      { value: "10M", label: "10 MB/s" },
      { value: "50M", label: "50 MB/s" },
      { value: "100M", label: "100 MB/s" },
    ],
    help: "A speed cap for this mount. Off uses the full connection. The bandwidth schedule can still add its own cap when the mount starts.",
  },
  {
    key: "tpslimit",
    group: "speed",
    label: "Requests per second",
    kind: "text",
    placeholder: "No cap",
    advanced: true,
    help: "Caps how fast rclone talks to the host. Use this when a provider rate-limits you.",
  },
  {
    key: "fast-list",
    group: "speed",
    label: "Fast list",
    kind: "bool",
    help: "Lists folders with fewer API calls when the remote supports it.",
  },
  {
    key: "network-mode",
    group: "access",
    label: "Network drive",
    kind: "bool",
    help: "Windows shows the mount as a network location. Some apps only see a drive letter this way.",
  },
  {
    key: "read-only",
    group: "access",
    label: "Read only",
    kind: "bool",
    help: "Blocks every write. Use this for a library you do not want to change.",
  },
  {
    key: "volname",
    group: "access",
    label: "Volume name",
    kind: "text",
    placeholder: "Windows default",
    help: "The name Windows shows for the drive. This is separate from the drive letter.",
  },
  {
    key: "links",
    group: "access",
    label: "Show link files as links",
    kind: "bool",
    advanced: true,
    help: "Translates rclone link files into links the mount can follow.",
  },
  {
    key: "vfs-case-insensitive",
    group: "access",
    label: "Case insensitive names",
    kind: "bool",
    advanced: true,
    help: "Treats File.txt and file.txt as the same file. Turn this on for Windows apps that ignore case.",
  },
  {
    key: "allow-other",
    group: "access",
    label: "Allow other Windows users",
    kind: "bool",
    advanced: true,
    help: "Lets other accounts on this PC open the mount.",
  },
  {
    key: "file-perms",
    group: "access",
    label: "File permissions",
    kind: "text",
    placeholder: "0666",
    advanced: true,
    help: "Permission bits rclone reports for files.",
  },
  {
    key: "dir-perms",
    group: "access",
    label: "Folder permissions",
    kind: "text",
    placeholder: "0777",
    advanced: true,
    help: "Permission bits rclone reports for folders.",
  },
  {
    key: "umask",
    group: "access",
    label: "Umask",
    kind: "text",
    placeholder: "002",
    advanced: true,
    help: "Applied to the file and folder permission bits.",
  },
  {
    key: "no-modtime",
    group: "reliability",
    label: "Skip modification times",
    kind: "bool",
    advanced: true,
    help: "Stops rclone from reading or writing file times. Faster on some clouds. Apps will see the wrong dates.",
  },
  {
    key: "no-checksum",
    group: "reliability",
    label: "Skip checksums",
    kind: "bool",
    advanced: true,
    help: "Skips checksum checks. Faster, with less certainty that a transfer matched.",
  },
  {
    key: "timeout",
    group: "reliability",
    label: "IO timeout",
    kind: "text",
    placeholder: "5m",
    advanced: true,
    help: "How long one read or write may run before rclone gives up.",
  },
  {
    key: "contimeout",
    group: "reliability",
    label: "Connect timeout",
    kind: "text",
    placeholder: "1m",
    advanced: true,
    help: "How long rclone waits to open a connection.",
  },
  {
    key: "retries",
    group: "reliability",
    label: "Retries",
    kind: "text",
    placeholder: "3",
    advanced: true,
    help: "How many times a failed operation is tried again.",
  },
  {
    key: "low-level-retries",
    group: "reliability",
    label: "Low-level retries",
    kind: "text",
    placeholder: "10",
    advanced: true,
    help: "Retries for a single HTTP request.",
  },
  {
    key: "log-level",
    group: "reliability",
    label: "Log level",
    kind: "choice",
    options: [
      { value: "", label: "Default" },
      { value: "NOTICE", label: "Notice" },
      { value: "INFO", label: "Info" },
      { value: "DEBUG", label: "Debug" },
    ],
    help: "Debug is the one to use when a mount fails and you need the reason. It is noisy.",
  },
  {
    key: "log-file",
    group: "reliability",
    label: "Log file",
    kind: "text",
    placeholder: "Off",
    help: "Writes rclone’s log to this file. Leave empty to keep logging off.",
  },
  {
    key: "rc",
    group: "status",
    label: "Remote control",
    kind: "bool",
    help: "Lets LiClone show live uploads for this mount. It listens on localhost only, with no password.",
  },
  {
    key: "rc-port",
    group: "status",
    label: "Remote control port",
    kind: "text",
    placeholder: "5572",
    help: "Local port for that status connection. Each mount that runs at the same time needs its own port.",
  },
  {
    key: "daemon",
    group: "status",
    label: "Detach process",
    kind: "bool",
    advanced: true,
    help: "Asks rclone to daemonize. LiClone already starts the mount without a console, so leave this off unless you need it.",
  },
];

export const MOUNT_PRESETS: {
  id: string;
  label: string;
  help: string;
  settings: Record<string, string>;
}[] = [
  {
    id: "everyday",
    label: "Everyday drive",
    help: "A normal Windows drive. Files stay on disk until the upload finishes.",
    settings: {
      "vfs-cache-mode": "writes",
      "dir-cache-time": "5m",
      "buffer-size": "32M",
      "vfs-write-back": "5s",
      transfers: "4",
      "network-mode": "1",
      rc: "1",
      "rc-port": "5572",
    },
  },
  {
    id: "video",
    label: "Video library",
    help: "Bigger reads and a long folder cache for playback.",
    settings: {
      "vfs-cache-mode": "full",
      "buffer-size": "64M",
      "vfs-read-chunk-size": "32M",
      "vfs-read-ahead": "128M",
      "dir-cache-time": "30m",
      "poll-interval": "15m",
      "network-mode": "1",
      "fast-list": "1",
      rc: "1",
    },
  },
  {
    id: "edit",
    label: "Edit files",
    help: "Full cache and a short wait before upload, for documents you save often.",
    settings: {
      "vfs-cache-mode": "full",
      "vfs-write-back": "1s",
      "vfs-cache-max-size": "50G",
      "dir-cache-time": "15s",
      transfers: "4",
      "network-mode": "1",
      rc: "1",
    },
  },
  {
    id: "browse",
    label: "Browse only",
    help: "Read-only, with no write cache.",
    settings: {
      "vfs-cache-mode": "minimal",
      "read-only": "1",
      "buffer-size": "16M",
      "dir-cache-time": "15m",
      "network-mode": "1",
      rc: "",
    },
  },
  {
    id: "lowdisk",
    label: "Low disk use",
    help: "Keeps the cache small and drops old files quickly.",
    settings: {
      "vfs-cache-mode": "minimal",
      "vfs-cache-max-size": "1G",
      "vfs-cache-max-age": "6h",
      "buffer-size": "16M",
      "dir-cache-time": "5m",
      "network-mode": "1",
    },
  },
];

export function matchingPreset(settings: Record<string, string>) {
  return (
    MOUNT_PRESETS.find((preset) =>
      Object.entries(preset.settings).every(([key, value]) => (settings[key] ?? "") === value),
    )?.id ?? ""
  );
}

const SKIP_FLAGS = new Set([
  "stats",
  "stats-file-name-length",
  "stats-log-level",
  "progress",
  "verbose",
  "quiet",
  "human-readable",
  "use-json-log",
]);

export function profileFromInstance(inst: RcloneInstance, existing: MountProfile[]): MountProfile {
  const remoteRaw = (inst.parsed.remote ?? "").trim();
  const colon = remoteRaw.indexOf(":");
  const remote = (colon >= 0 ? remoteRaw.slice(0, colon) : remoteRaw).replace(/:$/, "");
  const remotePath = colon >= 0 ? remoteRaw.slice(colon + 1).replace(/^[/\\]+/, "") : "";
  const mountPoint = inst.parsed.mountPoint ?? "";
  const known = new Set(MOUNT_SETTINGS.map((item) => item.key));
  const settings: Record<string, string> = {};
  const extra: string[] = [];

  for (const [rawKey, rawValue] of Object.entries(inst.parsed.flags)) {
    const name = rawKey.replace(/^-+/, "");
    if (!name || SKIP_FLAGS.has(name)) continue;
    if (name === "rc" || name === "rc-no-auth") {
      settings.rc = "1";
      continue;
    }
    if (name === "rc-addr") {
      settings.rc = "1";
      const port = String(rawValue ?? "").split(":").pop() ?? "";
      if (/^\d+$/.test(port)) settings["rc-port"] = port;
      continue;
    }
    const value = rawValue == null || rawValue === "" ? "1" : rawValue;
    if (known.has(name)) settings[name] = value;
    else extra.push(value === "1" ? `--${name}` : `--${name} ${/\s/.test(value) ? `"${value}"` : value}`);
  }
  if (inst.vfsCacheMode) settings["vfs-cache-mode"] = inst.vfsCacheMode;
  if (inst.cacheDir) settings["cache-dir"] = inst.cacheDir;
  if (inst.rcAddr) {
    settings.rc = "1";
    const port = inst.rcAddr.split(":").pop() ?? "";
    if (/^\d+$/.test(port)) settings["rc-port"] = port;
  }

  const match = mountPoint
    ? existing.find((profile) => profile.mountPoint.trim().toLowerCase() === mountPoint.trim().toLowerCase())
    : undefined;
  const base = match ? hydrateMount(match) : { ...emptyMount(), settings: {} };
  const name = match?.name || remote || mountPoint || "Mount";
  return mirrorMount({
    ...base,
    id: match?.id ?? "",
    name,
    remote: remote || base.remote,
    remotePath,
    mountPoint: mountPoint || base.mountPoint,
    extraArgs: extra.join(" "),
    startOnLogin: match?.startOnLogin ?? false,
    settings: match ? { ...base.settings, ...settings } : settings,
  });
}

export function hydrateMount(profile: MountProfile): MountProfile {
  if (Object.keys(profile.settings ?? {}).length) return { ...profile, settings: profile.settings };
  return {
    ...profile,
    settings: {
      "vfs-cache-mode": profile.vfsCacheMode || "full",
      "cache-dir": profile.cacheDir || "",
      volname: profile.volName || "",
      "network-mode": profile.networkMode ? "1" : "",
      "read-only": profile.readOnly ? "1" : "",
      rc: profile.rcEnabled ? "1" : "",
      "rc-port": String(profile.rcPort || 5572),
    },
  };
}

export function mirrorMount(profile: MountProfile): MountProfile {
  const settings = profile.settings ?? {};
  const port = Number(settings["rc-port"] || profile.rcPort || 5572);
  return {
    ...profile,
    settings,
    vfsCacheMode: settings["vfs-cache-mode"] || profile.vfsCacheMode,
    cacheDir: settings["cache-dir"] ?? profile.cacheDir,
    volName: settings.volname ?? profile.volName,
    networkMode: settings["network-mode"] != null ? settings["network-mode"] === "1" : profile.networkMode,
    readOnly: settings["read-only"] != null ? settings["read-only"] === "1" : profile.readOnly,
    rcEnabled: settings.rc != null ? settings.rc === "1" : profile.rcEnabled,
    rcPort: Number.isFinite(port) && port > 0 ? port : 5572,
  };
}
