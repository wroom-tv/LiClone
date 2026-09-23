import { FormEvent, useEffect, useRef, useState } from "react";
import { Button, Field, Input, Switch, Textarea } from "./sealed/ui";
import { MenuSelect } from "./menu-select";
import { chooseFolder } from "./folder";
import { MOUNT_GROUPS, MOUNT_PRESETS, MOUNT_SETTINGS, matchingPreset, mirrorMount, type MountSetting } from "./mountCatalog";
import type { CacheAdvice, MountProfile } from "./types";

export function MountEditor({
  draft,
  remotes,
  busy,
  preview,
  advice,
  onChange,
  onSubmit,
}: {
  draft: MountProfile;
  remotes: string[];
  busy: boolean;
  preview: string;
  advice: CacheAdvice | null;
  onChange: (profile: MountProfile) => void;
  onSubmit: (event: FormEvent) => void;
}) {
  const [full, setFull] = useState(false);
  const [customKey, setCustomKey] = useState("");
  const [customValue, setCustomValue] = useState("");
  const known = new Set(MOUNT_SETTINGS.map((item) => item.key));
  const settings = draft.settings ?? {};

  function patch(partial: Partial<MountProfile>) {
    onChange(mirrorMount({ ...draft, ...partial }));
  }

  function setSetting(key: string, value: string) {
    patch({ settings: { ...settings, [key]: value } });
  }

  const extras = Object.entries(settings).filter(([key]) => !known.has(key));
  const applied = matchingPreset(settings);
  const appliedHelp = MOUNT_PRESETS.find((preset) => preset.id === applied)?.help;

  return (
    <form className="form mount-editor" onSubmit={onSubmit}>
      <h2>{draft.id ? "Edit mount" : "New mount"}</h2>
      <p className="help-note">
        The highlighted preset is the one these settings match. Change a slider and the highlight goes away until the values line up again.
      </p>
      <div className="preset-grid">
        {MOUNT_PRESETS.map((preset) => (
          <Button
            key={preset.id}
            type="button"
            variant="ghost"
            size="sm"
            className={applied === preset.id ? "preset-on" : ""}
            onClick={() => patch({ settings: { ...settings, ...preset.settings } })}
          >
            {preset.label}
          </Button>
        ))}
      </div>
      <p className="help-note preset-help">{appliedHelp || "These settings do not match a preset."}</p>
      <div className="pair">
        <Field label="Name" hint="Shown in the mount list.">
          <Input value={draft.name} required placeholder="Work drive" onChange={(e) => patch({ name: e.target.value })} />
        </Field>
        <Field label="Remote" hint="The rclone remote this drive uses.">
          <MenuSelect
            value={draft.remote}
            placeholder="Choose a remote"
            options={remotes.map((remote) => ({ value: remote, label: remote }))}
            onChange={(remote) => patch({ remote })}
          />
        </Field>
      </div>
      <div className="pair">
        <Field label="Folder on the remote" hint="Leave empty for the whole remote.">
          <Input value={draft.remotePath} placeholder="Documents" onChange={(e) => patch({ remotePath: e.target.value })} />
        </Field>
        <Field label="Mount point" hint="A drive letter such as R:, or a folder.">
          <div className="path-row">
            <Input
              value={draft.mountPoint}
              required
              placeholder="R:"
              onChange={(e) => patch({ mountPoint: e.target.value })}
            />
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => {
                void chooseFolder().then((folder) => {
                  if (folder) patch({ mountPoint: folder });
                });
              }}
            >
              Browse
            </Button>
          </div>
        </Field>
      </div>
      {MOUNT_GROUPS.map((group) => {
        const items = MOUNT_SETTINGS.filter((item) => item.group === group.id && !item.advanced);
        if (!items.length) return null;
        return (
          <section key={group.id} className="setting-block">
            <h2>{group.title}</h2>
            <SettingRows items={items} settings={settings} setSetting={setSetting} advice={advice} />
          </section>
        );
      })}
      <button type="button" className={`disclose ${full ? "open" : ""}`} aria-expanded={full} onClick={() => setFull((on) => !on)}>
        <i />
        <span>
          <strong>Extra flags</strong>
          <span className="disclose-note">Permissions, retries, logs, and any rclone flag that is not in the list above.</span>
        </span>
      </button>
      {full && (
        <div className="disclose-body">
      {MOUNT_GROUPS.map((group) => {
        const items = MOUNT_SETTINGS.filter((item) => item.group === group.id && item.advanced);
        if (!items.length) return null;
        return (
          <section key={group.id} className="setting-block">
            <h2>{group.title}</h2>
            <SettingRows items={items} settings={settings} setSetting={setSetting} advice={advice} />
          </section>
        );
      })}
      {extras.length > 0 && (
        <section className="setting-block">
          <h2>Custom flags</h2>
          {extras.map(([key, value]) => (
            <Field key={key} label={`--${key}`} hint="Passed through to rclone as you typed it.">
              <div className="path-row">
                <Input value={value} onChange={(e) => setSetting(key, e.target.value)} />
                <Button
                  type="button"
                  variant="danger"
                  size="sm"
                  onClick={() => {
                    const next = { ...settings };
                    delete next[key];
                    patch({ settings: next });
                  }}
                >
                  Remove
                </Button>
              </div>
            </Field>
          ))}
        </section>
      )}
      <Field
        label="Add a custom flag"
        hint="Any rclone mount flag that is not listed above. Name without dashes, plus a value. Example: drive-chunk-size and 64M. A flag with no value goes in raw arguments."
      >
        <div className="path-row">
          <Input value={customKey} placeholder="drive-chunk-size" onChange={(e) => setCustomKey(e.target.value)} />
          <Input value={customValue} placeholder="64M" onChange={(e) => setCustomValue(e.target.value)} />
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              const key = customKey.trim().replace(/^-+/, "");
              if (!key) return;
              setSetting(key, customValue.trim());
              setCustomKey("");
              setCustomValue("");
            }}
          >
            Add
          </Button>
        </div>
      </Field>
      <Field
        label="Raw arguments"
        hint="Added at the end of the command, after the settings above. Repeat a flag here if you need it to win."
      >
        <Textarea
          value={draft.extraArgs}
          placeholder="--drive-acknowledge-abuse"
          onChange={(e) => patch({ extraArgs: e.target.value })}
        />
      </Field>
        </div>
      )}
      <div className="mount-foot">
        <Switch
          checked={draft.startOnLogin}
          label="Start when I log in"
          description="Writes a hidden script in the Windows Startup folder for this profile."
          onChange={(on) => patch({ startOnLogin: on })}
        />
        <Button variant="primary" disabled={busy} type="submit">
          Save profile
        </Button>
        {preview && (
          <div>
            <h2>Command</h2>
            <div className="preview">{preview}</div>
          </div>
        )}
      </div>
    </form>
  );
}

function canPair(kind: MountSetting["kind"]) {
  return kind === "menu" || kind === "bool" || kind === "text";
}

function pairedRows(items: MountSetting[]) {
  const rows: MountSetting[][] = [];
  for (let i = 0; i < items.length; i += 1) {
    const next = items[i + 1];
    if (next && canPair(items[i].kind) && items[i].kind === next.kind) {
      rows.push([items[i], next]);
      i += 1;
    } else {
      rows.push([items[i]]);
    }
  }
  return rows;
}

function SettingRows({
  items,
  settings,
  setSetting,
  advice,
}: {
  items: MountSetting[];
  settings: Record<string, string>;
  setSetting: (key: string, value: string) => void;
  advice: CacheAdvice | null;
}) {
  return (
    <>
      {pairedRows(items).map((row) => (
        <div key={row.map((item) => item.key).join("-")} className={row.length === 2 ? "pair" : "pair solo"}>
          {row.map((item) => (
            <SettingControl
              key={item.key}
              item={item}
              value={settings[item.key] ?? ""}
              advice={item.key === "vfs-cache-max-size" ? advice : null}
              onChange={(value) => setSetting(item.key, value)}
            />
          ))}
        </div>
      ))}
    </>
  );
}

function SettingControl({
  item,
  value,
  advice,
  onChange,
}: {
  item: MountSetting;
  value: string;
  advice: CacheAdvice | null;
  onChange: (value: string) => void;
}) {
  const hint = advice ? `${item.help} Recommended for this PC: ${advice.label}. ${advice.reason}` : item.help;
  if (item.kind === "bool") {
    return <Switch checked={value === "1"} label={item.label} description={item.help} onChange={(on) => onChange(on ? "1" : "")} />;
  }
  const options = item.kind === "range" || item.kind === "menu" ? withBlank(item.options ?? []) : item.options ?? [];
  return (
    <Field label={item.label} hint={hint}>
      {item.kind === "choice" ? (
        <ChoiceBar value={value} options={options} onChange={onChange} />
      ) : item.kind === "range" ? (
        <RangeBar value={value} options={options} onChange={onChange} />
      ) : item.kind === "menu" ? (
        <MenuSelect value={value} placeholder="Default" options={withCurrent(value, options)} onChange={onChange} />
      ) : (
        <div className="path-row">
          <Input value={value} placeholder={item.placeholder || "Default"} onChange={(e) => onChange(e.target.value)} />
          {item.kind === "folder" && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => {
                void chooseFolder().then((folder) => {
                  if (folder) onChange(folder);
                });
              }}
            >
              Browse
            </Button>
          )}
        </div>
      )}
    </Field>
  );
}

function withBlank(options: { value: string; label: string }[]) {
  if (!options.length || options.some((option) => option.value === "")) return options;
  return [{ value: "", label: "Default" }, ...options];
}

function withCurrent(value: string, options: { value: string; label: string }[]) {
  if (!value || options.some((option) => option.value === value)) return options;
  return [{ value, label: value }, ...options];
}

function sizeBytes(token: string) {
  const match = /^(\d+(?:\.\d+)?)([KMGT])?$/i.exec(token.trim());
  if (!match) return null;
  const amount = Number(match[1]);
  const unit = (match[2] || "").toUpperCase();
  const mul = unit === "K" ? 1024 : unit === "M" ? 1024 ** 2 : unit === "G" ? 1024 ** 3 : unit === "T" ? 1024 ** 4 : 1;
  return amount * mul;
}

function indexFor(raw: string, options: { value: string; label: string }[]) {
  const exact = options.findIndex((option) => option.value === raw);
  if (exact >= 0) return exact;
  const target = sizeBytes(raw);
  if (target == null) return 0;
  let best = 0;
  let bestDist = Number.POSITIVE_INFINITY;
  options.forEach((option, index) => {
    const bytes = sizeBytes(option.value);
    if (bytes == null) return;
    const dist = Math.abs(bytes - target);
    if (dist < bestDist) {
      best = index;
      bestDist = dist;
    }
  });
  return best;
}

function ChoiceBar({
  value,
  options,
  onChange,
}: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
}) {
  return (
    <div className="choice-bar" role="radiogroup">
      {options.map((option) => (
        <button
          type="button"
          key={option.value || option.label}
          className={option.value === value ? "on" : ""}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

function RangeBar({
  value,
  options,
  onChange,
}: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
}) {
  function displayFor(raw: string) {
    const match = options.find((option) => option.value === raw);
    if (match) return match.label || "Default";
    if (!raw.trim()) return "Default";
    return raw;
  }

  const exact = indexFor(value, options);
  const [at, setAt] = useState(exact);
  const [text, setText] = useState(() => displayFor(value));
  const dragging = useRef(false);
  const editing = useRef(false);
  const atRef = useRef(at);
  atRef.current = at;

  useEffect(() => {
    if (dragging.current || editing.current) return;
    if (exact >= 0) setAt(exact);
    setText(displayFor(value));
  }, [exact, value]);

  function commitIndex(index: number) {
    dragging.current = false;
    const next = options[index]?.value ?? "";
    setText(displayFor(next));
    if (next !== value) onChange(next);
  }

  function commitText() {
    editing.current = false;
    const typed = text.trim();
    const byLabel = options.find((option) => option.label.toLowerCase() === typed.toLowerCase());
    const next = byLabel ? byLabel.value : typed;
    setText(displayFor(next));
    if (next !== value) onChange(next);
  }

  return (
    <div className="range-bar">
      <input
        type="range"
        min={0}
        max={Math.max(options.length - 1, 0)}
        value={at}
        aria-valuetext={text || value}
        onPointerDown={(event) => {
          dragging.current = true;
          event.currentTarget.setPointerCapture(event.pointerId);
        }}
        onInput={(event) => {
          const index = Number(event.currentTarget.value);
          setAt(index);
          setText(displayFor(options[index]?.value ?? ""));
        }}
        onPointerUp={(event) => commitIndex(Number(event.currentTarget.value))}
        onKeyUp={(event) => commitIndex(Number(event.currentTarget.value))}
        onLostPointerCapture={() => {
          if (dragging.current) commitIndex(atRef.current);
        }}
      />
      <input
        className="range-value"
        value={text}
        placeholder="Default"
        spellCheck={false}
        aria-label="Custom value"
        onFocus={() => {
          editing.current = true;
        }}
        onChange={(event) => setText(event.target.value)}
        onBlur={commitText}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            event.currentTarget.blur();
          }
        }}
      />
    </div>
  );
}
