import { useEffect, useState } from "react";
import { Button, Field, Input, Modal, Switch } from "./sealed/ui";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { MenuSelect } from "./menu-select";
import { bytes } from "./format";
import type { BandwidthSchedule, CachePreview, HistoryEvent } from "./types";

export function SchedulePanel() {
  const [schedule, setSchedule] = useState<BandwidthSchedule | null>(null);
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    void api.loadBandwidth().then(setSchedule).catch((e) => setStatus(String(e)));
  }, []);

  if (!schedule) return <div className="card">Loading schedule...</div>;

  async function save(next: BandwidthSchedule) {
    setSchedule(next);
    const saved = await api.saveBandwidth(next);
    setSchedule(saved);
    setStatus(await api.applyBandwidth());
  }

  return (
    <div className="card">
      <h2>Bandwidth schedule</h2>
      <p className="empty">
        The limit from the latest time that has already passed is the one rclone uses. off means no cap.
        New mounts pick this up. Running mounts with remote control get the current limit.
      </p>
      <Switch
        checked={schedule.enabled}
        label="Use this schedule"
        onChange={(on) => void save({ ...schedule, enabled: on })}
      />
      {schedule.rules.map((rule, index) => (
        <div className="form" key={index} style={{ marginTop: 10 }}>
          <div className="field">
            <label>From</label>
            <input
              value={rule.start}
              placeholder="08:00"
              onChange={(e) => {
                const rules = schedule.rules.slice();
                rules[index] = { ...rule, start: e.target.value };
                setSchedule({ ...schedule, rules });
              }}
            />
          </div>
          <div className="field">
            <label>Limit</label>
            <input
              value={rule.limit}
              placeholder="2M or off"
              onChange={(e) => {
                const rules = schedule.rules.slice();
                rules[index] = { ...rule, limit: e.target.value };
                setSchedule({ ...schedule, rules });
              }}
            />
          </div>
        </div>
      ))}
      <div className="actions" style={{ marginTop: 12 }}>
        <Button variant="primary" size="sm" onClick={() => void save(schedule)}>
          Save schedule
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() =>
            setSchedule({
              ...schedule,
              rules: [...schedule.rules, { start: "", limit: "" }],
            })
          }
        >
          Add time
        </Button>
      </div>
      {status && <p className="empty">{status}</p>}
    </div>
  );
}

export function HistoryPanel({ events }: { events: HistoryEvent[] }) {
  const ordered = events.slice().reverse();
  return (
    <div className="card">
      <h2>History</h2>
      {!ordered.length && (
        <p className="empty">
          Uploads appear here after the file is no longer waiting on disk.
          Failures come from rclone logs and remote-control errors.
        </p>
      )}
      <table className="table">
        <tbody>
          {ordered.slice(0, 40).map((event, i) => (
            <tr key={`${event.at}-${event.name}-${i}`}>
              <td>
                <span className={`chip ${event.status === "failed" ? "warn" : "ok"}`}>
                  {event.status}
                </span>
              </td>
              <td>
                <div className="mono">{event.name}</div>
                <div className="mono">{event.detail}</div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

type ClearMode = "clean" | "cloud" | "old";

export function CacheClearModal({
  onClose,
  onCleared,
}: {
  onClose: () => void;
  onCleared: () => void;
}) {
  const [mode, setMode] = useState<ClearMode>("cloud");
  const [olderDays, setOlderDays] = useState(7);
  const [purging, setPurging] = useState(false);
  const [note, setNote] = useState("Nothing is deleted until you confirm.");
  const [preview, setPreview] = useState<CachePreview | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<CachePreview>("cache-purge", (event) => {
      setPreview(event.payload);
      if (event.payload.message) setNote(event.payload.message);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  return (
    <Modal
      wide
      eyebrow="Cache"
      title="Clear cache"
      subtitle="Uploads that are still on this PC and not on the cloud are always kept."
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Close
          </Button>
          {purging && (
            <Button variant="ghost" onClick={() => void api.cancelCacheClear()}>
              Stop
            </Button>
          )}
          <Button
            variant="primary"
            disabled={purging}
            onClick={() => {
              setPurging(true);
              setError(null);
              setPreview(null);
              setNote("Scanning in the background. You can keep using the window.");
              void api
                .previewCacheClear(mode, olderDays)
                .then((report) => {
                  setPreview(report);
                  if (report.errors.length) setError(report.errors.join("\n"));
                })
                .catch((err) => setError(String(err)))
                .finally(() => setPurging(false));
            }}
          >
            {purging ? "Working..." : "Preview"}
          </Button>
          <Button
            variant="danger"
            disabled={purging || !preview || preview.files === 0 || preview.mode === "delete"}
            onClick={() => {
              setPurging(true);
              setError(null);
              void api
                .applyCacheClear()
                .then((report) => {
                  setPreview(report);
                  setNote(report.message ?? "Deleted the previewed files.");
                  onCleared();
                })
                .catch((err) => setError(String(err)))
                .finally(() => setPurging(false));
            }}
          >
            Delete these files
          </Button>
        </>
      }
    >
      <Field
        label="What to clear"
        hint={
          mode === "cloud"
            ? "Runs rclone check --size-only --one-way for each cached remote. A file is listed when its size matches the cloud. Files still waiting to upload stay."
            : mode === "old"
              ? "Read cache that has not been used recently. Uploads stay on disk."
              : "Removes cached reads, including files that were only partly downloaded. Files that still need to upload stay on disk."
        }
      >
        <MenuSelect
          value={mode}
          options={[
            { value: "clean", label: "Read cache that is not an upload" },
            { value: "cloud", label: "Cached files already in the cloud" },
            { value: "old", label: "Untouched read cache" },
          ]}
          onChange={(value) => setMode(value as ClearMode)}
        />
      </Field>
      {mode === "old" && (
        <Field label="Not touched in the last">
          <Input
            type="number"
            min={1}
            value={olderDays}
            onChange={(e) => setOlderDays(Number(e.target.value) || 1)}
          />
        </Field>
      )}
      <p className="empty">{note}</p>
      {error && <div className="toast">{error}</div>}
      {preview && (
        <>
          <div className="metrics">
            <div className="metric">
              <div className="k">Would remove</div>
              <div className="v">{preview.files}</div>
            </div>
            <div className="metric">
              <div className="k">Space</div>
              <div className="v">{bytes(preview.bytes)}</div>
            </div>
            <div className="metric">
              <div className="k">Uploads kept</div>
              <div className="v">{preview.keptUploads}</div>
            </div>
            <div className="metric">
              <div className="k">Other kept</div>
              <div className="v">{preview.keptOther}</div>
            </div>
          </div>
          <table className="table">
            <thead>
              <tr>
                <th>File</th>
                <th>Size</th>
              </tr>
            </thead>
            <tbody>
              {preview.sample.map((file) => (
                <tr key={file.path}>
                  <td className="mono">{file.path}</td>
                  <td className="mono">{bytes(file.bytes)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {preview.files > preview.sample.length && (
            <p className="empty">Showing {preview.sample.length} of {preview.files} files.</p>
          )}
        </>
      )}
    </Modal>
  );
}
