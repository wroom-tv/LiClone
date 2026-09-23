export type ParsedRclone = {
  binary: string;
  verb: string;
  remote: string | null;
  mountPoint: string | null;
  flags: Record<string, string | null>;
  positional: string[];
  raw: string[];
};

export type RcloneInstance = {
  pid: number;
  name: string;
  exe: string | null;
  cwd: string | null;
  cmd: string[];
  cmdLine: string;
  parsed: ParsedRclone;
  rcAddr: string | null;
  rcUser: string | null;
  rcHasPass: boolean;
  cacheDir: string | null;
  vfsCacheMode: string | null;
  cpu: number;
  memory: number;
  runTimeSecs: number | null;
  status: string;
  parentPid: number | null;
};

export type CacheAdvice = {
  value: string;
  label: string;
  reason: string;
};

export type SetupState = {
  completed: boolean;
  autostart: boolean;
};

export type RcloneInfo = {
  found: boolean;
  path: string | null;
  version: string | null;
  compatible: boolean;
  error: string | null;
};

export type TransferItem = {
  name: string;
  size: number;
  bytes: number;
  percentage: number;
  speed: number;
  speedAvg: number;
  eta: number | null;
  group: string | null;
  srcFs: string | null;
  dstFs: string | null;
  phase?: string | null;
  addr?: string;
};

export type RcProbe = {
  addr: string;
  ok: boolean;
  error: string | null;
  stats: Record<string, unknown> | null;
  vfs: Record<string, unknown> | null;
  transferring: TransferItem[];
  transferred: Record<string, unknown>[];
  options: Record<string, unknown> | null;
};

export type CacheFile = {
  path: string;
  relative: string;
  size: number;
  mtime: number;
  recent: boolean;
};

export type CacheBucket = {
  remote: string;
  files: number;
  bytes: number;
  recentFiles: number;
  recentBytes: number;
};

export type CacheRoot = {
  path: string;
  exists: boolean;
  bytes: number;
  files: number;
  dirs: number;
  buckets: CacheBucket[];
  largest: CacheFile[];
  recent: CacheFile[];
  source: string;
};

export type CacheReport = {
  roots: CacheRoot[];
  totalBytes: number;
  totalFiles: number;
};

export type ObservedTransfers = {
  transferring: TransferItem[];
  queued: number;
  uploading: number;
  dirtyBytes: number;
  recentlyDone: TransferItem[];
};

export type HistoryEvent = {
  at: number;
  name: string;
  size: number;
  status: string;
  detail: string;
};

export type BandwidthRule = { start: string; limit: string };

export type BandwidthSchedule = {
  enabled: boolean;
  rules: BandwidthRule[];
};

export type CacheSample = { path: string; bytes: number };

export type CachePreview = {
  mode: string;
  files: number;
  bytes: number;
  keptUploads: number;
  keptOther: number;
  sample: CacheSample[];
  errors: string[];
  cancelled: boolean;
  message?: string;
};

export type MountProfile = {
  id: string;
  name: string;
  remote: string;
  remotePath: string;
  mountPoint: string;
  vfsCacheMode: string;
  cacheDir: string;
  extraArgs: string;
  rcEnabled: boolean;
  rcPort: number;
  networkMode: boolean;
  readOnly: boolean;
  volName: string;
  startOnLogin: boolean;
  background: boolean;
  settings: Record<string, string>;
};

export const emptyMount = (): MountProfile => ({
  id: "",
  name: "",
  remote: "",
  remotePath: "",
  mountPoint: "",
  vfsCacheMode: "full",
  cacheDir: "",
  extraArgs: "",
  rcEnabled: true,
  rcPort: 5572,
  networkMode: true,
  readOnly: false,
  volName: "",
  startOnLogin: false,
  background: true,
  settings: {
    "vfs-cache-mode": "writes",
    "dir-cache-time": "5m",
    "buffer-size": "32M",
    "vfs-write-back": "5s",
    "transfers": "4",
    "network-mode": "1",
    rc: "1",
    "rc-port": "5572",
  },
});

export type ProviderOption = {
  name: string;
  help: string;
  defaultValue: string;
  required: boolean;
  password: boolean;
  sensitive: boolean;
  advanced: boolean;
  type: string;
  examples: { value: string; help: string }[];
};

export type ProviderInfo = {
  name: string;
  description: string;
  options: ProviderOption[];
};

export type RemoteConfig = {
  name: string;
  kind: string;
  values: Record<string, string>;
};

export type RemoteParamWrite = {
  key: string;
  value: string;
  obscure: boolean;
};
