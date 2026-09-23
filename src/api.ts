import { invoke } from "@tauri-apps/api/core";
import type {
  BandwidthSchedule,
  CacheAdvice,
  CacheReport,
  HistoryEvent,
  MountProfile,
  ObservedTransfers,
  CachePreview,
  RcProbe,
  RcloneInfo,
  RcloneInstance,
  SetupState,
  ProviderInfo,
  RemoteConfig,
  RemoteParamWrite,
} from "./types";

export const api = {
  rcloneInfo: () => invoke<RcloneInfo>("rclone_info"),
  applySignedUpdate: (manifestUrl: string) =>
    invoke<void>("apply_signed_update", { manifestUrl }),
  installRclone: () => invoke<RcloneInfo>("install_rclone"),
  listRemotes: () => invoke<string[]>("list_remotes"),
  listInstances: () => invoke<RcloneInstance[]>("list_instances"),
  stopInstance: (pid: number) => invoke<void>("stop_instance", { pid }),
  probeRc: (targets: { addr: string; user?: string | null; pass?: string | null }[]) =>
    invoke<RcProbe[]>("probe_rc", { targets }),
  observeTransfers: () => invoke<ObservedTransfers>("observe_transfers"),
  analyzeCache: () => invoke<CacheReport>("analyze_cache"),
  previewCacheClear: (mode: string, olderThanDays: number) =>
    invoke<CachePreview>("preview_cache_clear", { mode, olderThanDays }),
  applyCacheClear: () => invoke<CachePreview>("apply_cache_clear"),
  cancelCacheClear: () => invoke<void>("cancel_cache_clear"),
  listMounts: () => invoke<MountProfile[]>("list_mounts"),
  saveMount: (profile: MountProfile) => invoke<MountProfile>("save_mount", { profile }),
  deleteMount: (id: string) => invoke<void>("delete_mount", { id }),
  startMount: (id: string) => invoke<number>("start_mount", { id }),
  previewMount: (profile: MountProfile) => invoke<string>("preview_mount", { profile }),
  cacheAdvice: () => invoke<CacheAdvice>("cache_advice"),
  listHistory: () => invoke<HistoryEvent[]>("list_history"),
  noteFailures: (messages: string[]) => invoke<HistoryEvent[]>("note_failures", { messages }),
  loadBandwidth: () => invoke<BandwidthSchedule>("load_bandwidth"),
  saveBandwidth: (schedule: BandwidthSchedule) =>
    invoke<BandwidthSchedule>("save_bandwidth", { schedule }),
  applyBandwidth: () => invoke<string>("apply_bandwidth"),
  createRemote: (draft: {
    name: string;
    kind: string;
    params: { key: string; value: string }[];
    obscure: string[];
  }) => invoke<string>("create_remote", { draft }),
  remoteProviders: () => invoke<ProviderInfo[]>("remote_providers"),
  remoteConfigs: () => invoke<RemoteConfig[]>("remote_configs"),
  saveRemote: (body: {
    name: string;
    previousName: string;
    kind: string;
    previousKind: string;
    create: boolean;
    params: RemoteParamWrite[];
    clear: string[];
  }) => invoke<string>("save_remote", { body }),
  deleteRemote: (name: string) => invoke<void>("delete_remote", { name }),
  reconnectRemote: (name: string) => invoke<string>("reconnect_remote", { name }),
  setupStatus: () => invoke<SetupState>("setup_status"),
  completeSetup: (autostart: boolean) => invoke<SetupState>("complete_setup", { autostart }),
};
