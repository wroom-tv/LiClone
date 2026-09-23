import { FormEvent, useEffect, useMemo, useState } from "react";
import { Button } from "./sealed/ui";
import { api } from "./api";
import { TabIcon } from "./icons";
import { MountEditor } from "./mount-form";
import { hydrateMount, mirrorMount, profileFromInstance } from "./mountCatalog";
import { ago, bytes, duration, eta, pct, rate } from "./format";
import type {
  CacheAdvice,
  CacheReport,
  HistoryEvent,
  MountProfile,
  ObservedTransfers,
  RcProbe,
  RcloneInfo,
  RcloneInstance,
  TransferItem,
} from "./types";
import { emptyMount } from "./types";
import { HistoryPanel, SchedulePanel, CacheClearModal } from "./panels";
import { RemotesPanel } from "./remotes-panel";
import { Setup } from "./setup";
import { TitleBar } from "./TitleBar";
import "./App.css";

type Tab = "instances" | "mounts" | "cache" | "uploads" | "transfers" | "remotes" | "schedule";

const TABS: Tab[] = ["instances", "mounts", "cache", "uploads", "transfers", "remotes", "schedule"];

function isRemoteFs(fs: string | null | undefined) {
  if (!fs || !fs.includes(":")) return false;
  const head = fs.split(":")[0] ?? "";
  if (head.length <= 1) return false;
  return !head.includes("\\") && !head.includes("/");
}

function sameFile(a: string, b: string) {
  const norm = (value: string) => value.split("\\").join("/").toLowerCase();
  const left = norm(a);
  const right = norm(b);
  return left === right || left.endsWith(`/${right}`) || right.endsWith(`/${left}`);
}

function cloudUpload(item: TransferItem) {
  return item.phase === "rc" && isRemoteFs(item.dstFs) && !isRemoteFs(item.srcFs);
}

function num(v: unknown): number {
  return typeof v === "number" ? v : 0;
}

export default function App() {
  const [tab, setTab] = useState<Tab>("instances");
  const [info, setInfo] = useState<RcloneInfo | null>(null);
  const [instances, setInstances] = useState<RcloneInstance[]>([]);
  const [probes, setProbes] = useState<RcProbe[]>([]);
  const [observed, setObserved] = useState<ObservedTransfers | null>(null);
  const [cache, setCache] = useState<CacheReport | null>(null);
  const [mounts, setMounts] = useState<MountProfile[]>([]);
  const [remotes, setRemotes] = useState<string[]>([]);
  const [history, setHistory] = useState<HistoryEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [draft, setDraft] = useState<MountProfile>(emptyMount());
  const [preview, setPreview] = useState("");
  const [busy, setBusy] = useState(false);
  const [startingId, setStartingId] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [clearOpen, setClearOpen] = useState(false);
  const [setupOpen, setSetupOpen] = useState(false);
  const [advice, setAdvice] = useState<CacheAdvice | null>(null);

  function freshMount() {
    const base = emptyMount();
    if (!advice) return base;
    return mirrorMount({
      ...base,
      settings: { ...base.settings, "vfs-cache-max-size": advice.value },
    });
  }

  function installRclone() {
    setInstalling(true);
    setError(null);
    void api
      .installRclone()
      .then(setInfo)
      .catch((err) => setError(String(err)))
      .finally(() => setInstalling(false));
  }

  async function refreshCore() {
    try {
      const [nextInfo, nextInstances] = await Promise.all([
        api.rcloneInfo(),
        api.listInstances(),
      ]);
      setInfo(nextInfo);
      setInstances(nextInstances);
      const targets: { addr: string; user?: string | null }[] = nextInstances
        .filter((i) => i.rcAddr)
        .map((i) => ({ addr: i.rcAddr as string, user: i.rcUser }));
      const parsedAddrs = new Set(targets.map((t) => t.addr));
      const probed = targets.length ? await api.probeRc(targets) : [];
      const live = probed.filter((p) => p.ok || parsedAddrs.has(p.addr));
      setProbes(live);
      setObserved(await api.observeTransfers());
      const failures = live.flatMap((probe) => {
        const last = probe.stats?.lastError;
        return typeof last === "string" && last.trim() ? [last] : [];
      });
      setHistory(failures.length ? await api.noteFailures(failures) : await api.listHistory());
    } catch (e) {
      setError(String(e));
    }
  }

  async function refreshAll() {
    await refreshCore();
    try {
      const [m, r] = await Promise.all([
        api.listMounts(),
        api.listRemotes().catch(() => [] as string[]),
      ]);
      setMounts(m.map(hydrateMount));
      setRemotes(r);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    void refreshAll();
    void api.setupStatus().then((state) => setSetupOpen(!state.completed)).catch(() => {});
    void api.cacheAdvice().then((next) => {
      setAdvice(next);
      setDraft((current) => {
        if (current.id || "vfs-cache-max-size" in (current.settings ?? {})) return current;
        return mirrorMount({
          ...current,
          settings: { ...current.settings, "vfs-cache-max-size": next.value },
        });
      });
    }).catch(() => {});
    const id = setInterval(() => void refreshCore(), 12000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    if (tab !== "cache" || cache) return;
    void api.analyzeCache().then(setCache).catch((e) => setError(String(e)));
  }, [tab, cache]);

  useEffect(() => {
    if (!draft.remote && remotes[0]) {
      setDraft((d) => ({ ...d, remote: remotes[0] }));
    }
  }, [remotes, draft.remote]);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void api.previewMount(draft).then((cmd) => {
        if (!cancelled) setPreview(cmd);
      }).catch(() => {
        if (!cancelled) setPreview("");
      });
    }, 400);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [draft]);

  const transferring = useMemo(() => {
    const pending = (observed?.transferring ?? []).filter(
      (item) => item.phase === "queued" || item.phase === "writing",
    );
    const rcUploads = probes.flatMap((probe) =>
      probe.transferring.filter(cloudUpload).map((item) => ({ ...item, addr: probe.addr })),
    );
    const merged = pending.map((item) => {
      const live = rcUploads.find((upload) => sameFile(upload.name, item.name));
      if (!live) return item;
      return {
        ...item,
        bytes: live.bytes,
        percentage: live.percentage,
        speed: live.speed,
        eta: live.eta,
        size: live.size > 0 ? live.size : item.size,
        phase: "uploading",
        dstFs: live.dstFs ?? item.dstFs,
      };
    });
    for (const upload of rcUploads) {
      if (!merged.some((item) => sameFile(item.name, upload.name))) {
        merged.push({ ...upload, phase: "uploading" });
      }
    }
    return merged;
  }, [probes, observed]);

  const speed = probes.reduce((s, p) => s + num(p.stats?.speed), 0);
  const bytesSoFar = probes.reduce((s, p) => s + num(p.stats?.bytes), 0);

  const usedPorts = new Set(
    instances
      .map((i) => i.rcAddr?.split(":").pop())
      .filter(Boolean)
      .map(Number),
  );

  async function adoptInstance(inst: RcloneInstance) {
    const profile = profileFromInstance(inst, mounts);
    setTab("mounts");
    setDraft(profile);
    setError(null);
    if (!profile.remote || !profile.mountPoint) {
      setError("This process has no remote or mount point yet. Fill those in, then save.");
      return;
    }
    setBusy(true);
    try {
      const saved = await api.saveMount(profile);
      setDraft(hydrateMount(saved));
      setMounts((await api.listMounts()).map(hydrateMount));
      confirmSaved(saved);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  function confirmSaved(saved: MountProfile) {
    const rcOn = saved.rcEnabled || saved.settings?.rc === "1";
    const message = rcOn
      ? `Saved ${saved.name}. Remote control is on port ${saved.rcPort}.`
      : `Saved ${saved.name}.`;
    setNotice(message);
    window.setTimeout(() => {
      setNotice((current) => (current === message ? null : current));
    }, 4000);
  }

  async function onSave(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      let port = draft.rcPort || 5572;
      if (!draft.id) {
        while (usedPorts.has(port)) port += 1;
      }
      const profile = mirrorMount({
        ...draft,
        rcPort: port,
        settings: { ...draft.settings, "rc-port": String(port) },
      });
      const saved = await api.saveMount(profile);
      setDraft(hydrateMount(saved));
      setMounts((await api.listMounts()).map(hydrateMount));
      confirmSaved(saved);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app">
      <TitleBar />
      {setupOpen && (
        <Setup
          info={info}
          installing={installing}
          error={error}
          onInstall={installRclone}
          onDone={() => setSetupOpen(false)}
        />
      )}
      <div className="frame">
      <aside className="rail">
        <nav>
          {TABS.map((key) => (
            <button
              key={key}
              className={tab === key ? "active" : ""}
              onClick={() => setTab(key)}
            >
              <TabIcon name={key} />
              <span>{key === "uploads" ? "active uploads" : key}</span>
            </button>
          ))}
        </nav>
        <div className="rail-foot">1.1.0</div>
      </aside>
      <div className="shell">
        <header className="top">
          <div>
            <p className="eyebrow">on disk, not on the cloud yet</p>
            <h1>LiClone</h1>
          </div>
          <div className="meta">
            <span>
              <i className={`dot ${info?.found ? "live" : "off"}`} />
              <b>
                {info?.found
                  ? info.version ?? info.path ?? "rclone"
                  : "rclone missing"}
              </b>
              {info?.found && info.compatible === false && (
                <span>Needs rclone 1.65 or newer</span>
              )}
              {!info?.found && (
                <Button
                  size="xs"
                  variant="primary"
                  disabled={installing}
                  onClick={installRclone}
                >
                  {installing ? "Installing…" : "Install rclone"}
                </Button>
              )}
            </span>
            <span>{instances.length} processes</span>
            <span>{rate(speed)}</span>
          </div>
        </header>
        <main className="content">
          {notice && <div className="toast ok">{notice}</div>}
          {error && <div className="toast">{error}</div>}
          {tab === "instances" && (
            <>
              <div className="metrics">
                <div className="metric">
                  <div className="k">Processes</div>
                  <div className="v">{instances.length}</div>
                </div>
                <div className="metric">
                  <div className="k">RC live</div>
                  <div className="v">{probes.filter((p) => p.ok).length}</div>
                </div>
                <div className="metric">
                  <div className="k">Throughput</div>
                  <div className="v">{rate(speed)}</div>
                </div>
                <div className="metric">
                  <div className="k">Moved</div>
                  <div className="v">{bytes(bytesSoFar)}</div>
                </div>
              </div>
              <div className="card">
                <h2>Running instances</h2>
                {!instances.length && (
                  <p className="empty">
                    No rclone processes right now. Create a mount on the next
                    tab, or start rclone with <span className="mono">--rc</span> so LiClone can
                    read live transfer stats.
                  </p>
                )}
                {instances.map((inst) => {
                  const probe = probes.find((p) => p.addr === inst.rcAddr);
                  return (
                    <article className="instance" key={inst.pid}>
                      <div>
                        <h3>
                          {inst.parsed.verb !== "unknown"
                            ? `${inst.parsed.verb} ${inst.parsed.remote ?? ""}`.trim()
                            : inst.name}
                        </h3>
                        <p>{inst.cmdLine || inst.exe || inst.name}</p>
                        <div className="chips">
                          <span className="chip">pid {inst.pid}</span>
                          {inst.parsed.mountPoint && (
                            <span className="chip blue">{inst.parsed.mountPoint}</span>
                          )}
                          {inst.vfsCacheMode && (
                            <span className="chip">vfs {inst.vfsCacheMode}</span>
                          )}
                          {inst.rcAddr && (
                            <span className={`chip ${probe?.ok ? "ok" : "warn"}`}>
                              rc {inst.rcAddr}
                              {probe?.ok ? " · connected" : " · not connected"}
                            </span>
                          )}
                          {!inst.rcAddr && (
                            <span className="chip warn">status off</span>
                          )}
                          <span className="chip">{bytes(inst.memory)} ram</span>
                          <span className="chip">up {duration(inst.runTimeSecs)}</span>
                        </div>
                      </div>
                      <div className="actions">
                        {inst.parsed.verb === "mount" && (
                          <Button variant="ghost" size="sm" onClick={() => void adoptInstance(inst)}>
                            {mounts.some(
                              (profile) =>
                                inst.parsed.mountPoint &&
                                profile.mountPoint.trim().toLowerCase() === inst.parsed.mountPoint.trim().toLowerCase(),
                            )
                              ? "Update mount"
                              : "Add to Mounts"}
                          </Button>
                        )}
                        <Button
                          variant="danger"
                          size="sm"
                          onClick={() =>
                            api.stopInstance(inst.pid).then(refreshCore).catch((e) => setError(String(e)))
                          }
                        >
                          Stop
                        </Button>
                      </div>
                    </article>
                  );
                })}
              </div>
            </>
          )}

          {tab === "mounts" && (
            <div className="row two">
              <div className="card">
                <MountEditor
                  draft={draft}
                  remotes={remotes}
                  busy={busy}
                  preview={preview}
                  advice={advice}
                  onChange={setDraft}
                  onSubmit={onSave}
                />
              </div>
              <div className="card">
                <h2>Saved mounts</h2>
                {!mounts.length && (
                  <p className="empty">
                    Profiles live in your app data. Saving with “start when I log in”
                    writes a hidden Startup script. Windows mounts need WinFSP.
                  </p>
                )}
                <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                  {mounts.map((m) => (
                    <div key={m.id}>
                      <button
                        type="button"
                        className={`list-btn ${draft.id === m.id ? "selected" : ""}`}
                        onClick={() => setDraft(hydrateMount(m))}
                      >
                        <span>
                          <strong>{m.name}</strong>
                          <div className="mono">
                            {m.remote}
                            {m.remotePath} → {m.mountPoint}
                          </div>
                        </span>
                        <span className="chip">{m.vfsCacheMode}</span>
                      </button>
                      <div className="actions" style={{ flexDirection: "row", marginTop: 6 }}>
                        <Button variant="ghost" size="sm" onClick={() => setDraft(hydrateMount(m))}>
                          Edit
                        </Button>
                        <Button
                          variant="primary"
                          size="sm"
                          disabled={startingId === m.id || mountRunning(m, instances)}
                          onClick={() => {
                            setStartingId(m.id);
                            setError(null);
                            api
                              .startMount(m.id)
                              .then(() => setTimeout(refreshCore, 800))
                              .catch((e) => setError(String(e)))
                              .finally(() => setStartingId(null));
                          }}
                        >
                          {mountRunning(m, instances) ? "Mounted" : "Mount"}
                        </Button>
                        <Button
                          variant="danger"
                          size="sm"
                          onClick={() =>
                            api
                              .deleteMount(m.id)
                              .then(async () => {
                                setMounts((await api.listMounts()).map(hydrateMount));
                                if (draft.id === m.id) setDraft(freshMount());
                              })
                              .catch((e) => setError(String(e)))
                          }
                        >
                          Delete
                        </Button>
                      </div>
                    </div>
                  ))}
                  <Button variant="ghost" size="sm" onClick={() => setDraft(freshMount())}>
                    New profile
                  </Button>
                </div>
              </div>
            </div>
          )}

          {tab === "cache" && (
            <>
              <div className="actions" style={{ marginBottom: 14 }}>
                <Button variant="primary" onClick={() => setClearOpen(true)}>
                  Clear cache
                </Button>
              </div>
              <div className="metrics">
                <div className="metric">
                  <div className="k">Cached</div>
                  <div className="v">{bytes(cache?.totalBytes ?? 0)}</div>
                </div>
                <div className="metric">
                  <div className="k">Files</div>
                  <div className="v">{cache?.totalFiles ?? 0}</div>
                </div>
                <div className="metric">
                  <div className="k">Roots</div>
                  <div className="v">{cache?.roots.length ?? 0}</div>
                </div>
                <div className="metric">
                  <div className="k">Recent writes</div>
                  <div className="v">
                    {cache?.roots.reduce((n, r) => n + r.recent.length, 0) ?? 0}
                  </div>
                </div>
              </div>
              {!cache?.roots.length && (
                <div className="card">
                  <p className="empty">No rclone cache directories found yet.</p>
                </div>
              )}
              {cache?.roots.map((root) => (
                <div className="card" key={root.path}>
                  <h2>{root.source}</h2>
                  <p className="mono empty" style={{ marginTop: -4 }}>
                    {root.path} · {bytes(root.bytes)} · {root.files} files
                  </p>
                  {root.buckets.length > 0 && (
                    <table className="table">
                      <thead>
                        <tr>
                          <th>Remote</th>
                          <th>Size</th>
                          <th>Files</th>
                          <th>Touched &lt; 30m</th>
                          <th></th>
                        </tr>
                      </thead>
                      <tbody>
                        {root.buckets.map((b) => {
                          const share = root.bytes ? (b.bytes / root.bytes) * 100 : 0;
                          return (
                            <tr key={b.remote}>
                              <td>{b.remote}</td>
                              <td className="mono">{bytes(b.bytes)}</td>
                              <td className="mono">{b.files}</td>
                              <td className="mono">
                                {b.recentFiles} · {bytes(b.recentBytes)}
                              </td>
                              <td style={{ width: 120 }}>
                                <div className="bar">
                                  <i style={{ width: `${share}%` }} />
                                </div>
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  )}
                  {root.recent.length > 0 && (
                    <>
                      <h2 style={{ marginTop: 18 }}>Recently written</h2>
                      <table className="table">
                        <thead>
                          <tr>
                            <th>File</th>
                            <th>Size</th>
                            <th>When</th>
                          </tr>
                        </thead>
                        <tbody>
                          {root.recent.map((f) => (
                            <tr key={f.path}>
                              <td className="mono">{f.relative}</td>
                              <td className="mono">{bytes(f.size)}</td>
                              <td>{ago(f.mtime)}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </>
                  )}
                </div>
              ))}
              <Button variant="ghost" size="sm" onClick={() => api.analyzeCache().then(setCache)}>
                Rescan cache
              </Button>
            </>
          )}

          {tab === "uploads" && (
            <div className="card">
              <h2>Active uploads</h2>
              <p className="empty">
                {transferring.length
                  ? `${transferring.length} file${transferring.length === 1 ? "" : "s"} still on this PC.`
                  : "Nothing is waiting to upload. A file shows up here only while rclone still has it marked dirty: saved on this PC, not on the remote yet."}
              </p>
              {transferring.length > 0 && (
              <table className="table">
                <thead>
                  <tr>
                    <th>File</th>
                    <th>Progress</th>
                    <th>Left</th>
                    <th>Speed</th>
                  </tr>
                </thead>
                <tbody>
                  {transferring.map((t, i) => {
                      const queued = t.phase === "queued";
                      const writing = t.phase === "writing";
                      const left = queued
                        ? t.size
                        : t.size > 0
                          ? Math.max(0, t.size - t.bytes)
                          : 0;
                      const phase = writing
                        ? "saving to disk"
                        : queued
                          ? "waiting to upload"
                          : "uploading";
                      return (
                        <tr key={`${t.name}-${i}`}>
                          <td>
                            <div>{t.name}</div>
                            <div className="mono" style={{ color: "var(--muted)" }}>
                              {phase}
                              {t.dstFs ? ` · ${t.dstFs}` : ""}
                            </div>
                          </td>
                          <td>
                            <div className="bar">
                              <i
                                style={{
                                  width: `${queued ? 0 : pct(t.percentage)}%`,
                                  background: queued ? "var(--hold)" : undefined,
                                }}
                              />
                            </div>
                            <div className="mono">
                              {queued
                                ? `${bytes(t.bytes)} on disk`
                                : writing
                                  ? `${pct(t.percentage).toFixed(0)}% saved locally`
                                  : `${pct(t.percentage).toFixed(0)}% · ${bytes(t.bytes)} / ${bytes(t.size)}`}
                            </div>
                          </td>
                          <td className="mono">
                            {queued ? "whole file" : bytes(left)}
                            <div>{queued ? "not started" : writing ? "local" : eta(t.eta)}</div>
                          </td>
                          <td className="mono">{queued || writing ? "—" : rate(t.speed)}</td>
                        </tr>
                      );
                    })}
                </tbody>
              </table>
              )}
            </div>
          )}

          {tab === "transfers" && (
            <div className="row two">
              <div className="card">
                <h2>On disk, not on the cloud</h2>
                {!transferring.some((item) => item.phase !== "uploading") && (
                  <p className="empty">
                    Nothing is waiting to upload. A file shows up here only while rclone
                    still has it marked dirty: saved on this PC, not on the remote yet.
                    Opening a file that only downloads into the cache does not appear.
                  </p>
                )}
                <table className="table">
                  <thead>
                    <tr>
                      <th>File</th>
                      <th>Progress</th>
                      <th>Left</th>
                      <th>Speed</th>
                    </tr>
                  </thead>
                  <tbody>
                    {transferring.filter((item) => item.phase !== "uploading").map((t, i) => {
                      const queued = t.phase === "queued";
                      const writing = t.phase === "writing";
                      const left = queued
                        ? t.size
                        : t.size > 0
                          ? Math.max(0, t.size - t.bytes)
                          : 0;
                      const phase = writing
                        ? "saving to disk"
                        : queued
                          ? "waiting to upload"
                          : "uploading";
                      return (
                        <tr key={`${t.name}-${i}`}>
                          <td>
                            <div>{t.name}</div>
                            <div className="mono" style={{ color: "var(--muted)" }}>
                              {phase}
                              {t.dstFs ? ` · ${t.dstFs}` : ""}
                            </div>
                          </td>
                          <td>
                            <div className="bar">
                              <i
                                style={{
                                  width: `${queued ? 0 : pct(t.percentage)}%`,
                                  background: queued ? "var(--hold)" : undefined,
                                }}
                              />
                            </div>
                            <div className="mono">
                              {queued
                                ? `${bytes(t.bytes)} on disk`
                                : writing
                                  ? `${pct(t.percentage).toFixed(0)}% saved locally`
                                  : `${pct(t.percentage).toFixed(0)}% · ${bytes(t.bytes)} / ${bytes(t.size)}`}
                            </div>
                          </td>
                          <td className="mono">
                            {queued ? "whole file" : bytes(left)}
                            <div>{queued ? "not started" : writing ? "local" : eta(t.eta)}</div>
                          </td>
                          <td className="mono">{queued || writing ? "—" : rate(t.speed)}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
              <div className="card">
                <h2>Writeback / VFS</h2>
                <div className="chips" style={{ marginBottom: 12 }}>
                  <span className="chip warn">waiting {observed?.queued ?? 0}</span>
                  <span className="chip blue">saving locally {observed?.uploading ?? 0}</span>
                  <span className="chip">{bytes(observed?.dirtyBytes ?? 0)} dirty</span>
                </div>
                {probes.filter((p) => p.ok).map((p) => {
                  const disk = (p.vfs?.diskCache ?? {}) as Record<string, unknown>;
                  return (
                    <div key={p.addr} className="instance">
                      <div>
                        <h3>{p.addr}</h3>
                        <div className="chips">
                          <span className="chip ok">
                            rc queued {num(disk.uploadsQueued)}
                          </span>
                          <span className="chip blue">
                            rc uploading {num(disk.uploadsInProgress)}
                          </span>
                          <span className="chip">{bytes(num(disk.bytesUsed))} cache</span>
                        </div>
                      </div>
                    </div>
                  );
                })}
                <h2 style={{ marginTop: 12 }}>Just finished</h2>
                <table className="table">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Size</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(observed?.recentlyDone ?? [])
                      .slice()
                      .reverse()
                      .slice(0, 12)
                      .map((t, i) => (
                        <tr key={`o-${i}`}>
                          <td className="mono">{t.name}</td>
                          <td className="mono">{bytes(t.size)}</td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {tab === "transfers" && <HistoryPanel events={history} />}

          {tab === "remotes" && (
            <RemotesPanel
              remotes={remotes}
              onCreated={() => {
                void api.listRemotes().then(setRemotes).catch((e) => setError(String(e)));
              }}
            />
          )}
          {tab === "schedule" && <SchedulePanel />}
        </main>
      </div>
      </div>
      {clearOpen && (
        <CacheClearModal
          onClose={() => setClearOpen(false)}
          onCleared={() => setCache(null)}
        />
      )}
    </div>
  );
}

function normPoint(value: string) {
  return value.trim().replace(/[\\/]+$/, "").toLowerCase();
}

function normSource(value: string) {
  const cleaned = value.trim().replace(/[\\/]+$/, "").replace(/\\/g, "/");
  const split = cleaned.indexOf(":");
  const remote = (split >= 0 ? cleaned.slice(0, split) : cleaned).toLowerCase();
  const path = (split >= 0 ? cleaned.slice(split + 1) : "").replace(/^\/+|\/+$/g, "").toLowerCase();
  return `${remote}:${path}`;
}

function mountRunning(profile: MountProfile, instances: RcloneInstance[]) {
  const point = normPoint(profile.mountPoint);
  const remote = profile.remote.replace(/:$/, "");
  const inside = profile.remotePath.replace(/^[/\\]+/, "").replace(/\\/g, "/");
  const source = normSource(inside ? `${remote}:${inside}` : `${remote}:`);
  return instances.some((inst) => {
    if (inst.parsed.verb !== "mount") return false;
    if (inst.parsed.mountPoint && normPoint(inst.parsed.mountPoint) === point) return true;
    return !!inst.parsed.remote && normSource(inst.parsed.remote) === source;
  });
}
