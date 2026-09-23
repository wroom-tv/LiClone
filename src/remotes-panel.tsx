import { FormEvent, useEffect, useMemo, useState } from "react";
import { Button, Field, Input } from "./sealed/ui";
import { MenuSelect } from "./menu-select";
import { api } from "./api";
import type { ProviderInfo, ProviderOption, RemoteConfig } from "./types";

function secretKey(key: string, option?: ProviderOption) {
  if (option?.password || option?.sensitive) return true;
  return /pass(word)?$|secret|token/i.test(key);
}

export function RemotesPanel({
  remotes,
  onCreated,
}: {
  remotes: string[];
  onCreated: () => void;
}) {
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [configs, setConfigs] = useState<RemoteConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [advanced, setAdvanced] = useState(false);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string>("");
  const [name, setName] = useState("");
  const [kind, setKind] = useState("drive");
  const [previousKind, setPreviousKind] = useState("");
  const [values, setValues] = useState<Record<string, string>>({});
  const [original, setOriginal] = useState<Record<string, string>>({});
  const [customKey, setCustomKey] = useState("");
  const [customValue, setCustomValue] = useState("");
  const [busy, setBusy] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const provider = providers.find((item) => item.name === kind);

  const visibleOptions = useMemo(() => {
    const options = provider?.options ?? [];
    return options.filter(
      (option) => (advanced || !option.advanced) && !(selected && Object.prototype.hasOwnProperty.call(original, option.name)),
    );
  }, [provider, advanced, selected, original]);

  const typeOptions = useMemo(() => {
    const q = query.trim().toLowerCase();
    return providers
      .filter((item) => {
        if (!q) return true;
        return item.name.toLowerCase().includes(q) || item.description.toLowerCase().includes(q);
      })
      .slice(0, 80)
      .map((item) => ({
        value: item.name,
        label: item.description ? `${item.name} — ${item.description}` : item.name,
      }));
  }, [providers, query]);

  async function load() {
    setLoading(true);
    const notes: string[] = [];
    const [providerResult, configResult] = await Promise.allSettled([
      api.remoteProviders(),
      api.remoteConfigs(),
    ]);
    if (providerResult.status === "fulfilled") setProviders(providerResult.value);
    else notes.push(String(providerResult.reason));
    if (configResult.status === "fulfilled") setConfigs(configResult.value);
    else notes.push(String(configResult.reason));
    setLoadError(notes.length ? notes.join(" ") : null);
    setLoading(false);
  }

  useEffect(() => {
    void load();
  }, []);

  function blank() {
    setSelected("");
    setName("");
    setPreviousKind("");
    setOriginal({});
    setValues({});
    setMessage(null);
    setKind(providers.find((item) => item.name === "drive")?.name ?? providers[0]?.name ?? "drive");
  }

  function openConfig(config: RemoteConfig) {
    setSelected(config.name);
    setName(config.name);
    setKind(config.kind || "");
    setPreviousKind(config.kind || "");
    setOriginal(config.values);
    const shown: Record<string, string> = {};
    const options = providers.find((item) => item.name === config.kind)?.options ?? [];
    for (const [key, value] of Object.entries(config.values)) {
      const option = options.find((item) => item.name === key);
      shown[key] = secretKey(key, option) ? "" : value;
    }
    setValues(shown);
    setMessage(null);
  }

  function setField(key: string, value: string) {
    setValues((current) => ({ ...current, [key]: value }));
  }

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setMessage(null);
    try {
      const creating = !selected;
      const typeChanged = !creating && previousKind !== kind;
      const params: { key: string; value: string; obscure: boolean }[] = [];
      const clear: string[] = [];
      const optionByName = new Map((provider?.options ?? []).map((option) => [option.name, option]));
      const keys = new Set([...Object.keys(original), ...Object.keys(values)]);
      for (const key of keys) {
        const option = optionByName.get(key);
        const next = (values[key] ?? "").trim();
        const prev = original[key] ?? "";
        const secret = secretKey(key, option);
        if (secret) {
          if (next) params.push({ key, value: next, obscure: Boolean(option?.password) });
          continue;
        }
        if (creating || typeChanged) {
          if (next) params.push({ key, value: next, obscure: false });
          continue;
        }
        if (next === prev) continue;
        if (!next && prev) clear.push(key);
        else if (next) params.push({ key, value: next, obscure: false });
      }
      const saved = await api.saveRemote({
        name,
        previousName: selected,
        kind,
        previousKind,
        create: creating || typeChanged,
        params,
        clear: typeChanged ? [] : clear,
      });
      setMessage(typeChanged ? `Rebuilt ${saved}` : `Saved ${saved}`);
      await load();
      onCreated();
      const fresh = (await api.remoteConfigs()).find((item) => item.name === name.trim());
      if (fresh) openConfig(fresh);
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    if (!selected) return;
    if (!window.confirm(`Delete the rclone remote ${selected}? This removes it from rclone’s config.`)) return;
    setBusy(true);
    setMessage(null);
    try {
      await api.deleteRemote(selected);
      setMessage(`Deleted ${selected}`);
      blank();
      await load();
      onCreated();
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function reconnect() {
    if (!selected) return;
    setBusy(true);
    setMessage("Waiting for rclone. Finish the browser sign-in if one opens.");
    try {
      const saved = await api.reconnectRemote(selected);
      setMessage(`Reconnected ${saved}`);
      await load();
      onCreated();
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  }

  const known = new Set((provider?.options ?? []).map((option) => option.name));
  const extras = Object.keys(values).filter((key) => !known.has(key) && (values[key] || original[key]));

  return (
    <div className="remote-page">
      <div className="card remote-list">
        <h2>Remotes</h2>
        <p className="help-note">Select a remote to edit the settings rclone has saved for it.</p>
        <Button variant="ghost" size="sm" onClick={blank}>
          New remote
        </Button>
        {loadError && <p className="empty">{loadError} LiClone expects rclone 1.65 or newer.</p>}
        {!configs.length && !remotes.length && <p className="empty">None yet.</p>}
        <div className="stack">
          {(configs.length
            ? configs
            : remotes.map((remote) => ({ name: remote.replace(/:$/, ""), kind: "", values: {} as Record<string, string> }))
          ).map((config) => (
              <button
                key={config.name}
                type="button"
                className={`list-btn ${selected === config.name ? "selected" : ""}`}
                onClick={() => openConfig(config)}
              >
                <span>
                  <strong>{config.name}</strong>
                  <div className="mono">{config.kind || "Edit settings"}</div>
                </span>
                <span className="chip">Edit</span>
              </button>
            ))}
        </div>
      </div>
      <div className="card mount-scroll">
        <h2>{selected ? `Edit ${selected}` : "New remote"}</h2>
        <p className="help-note">
          {selected
            ? "The settings below are already saved in rclone. Change a value and choose Save remote. Passwords and tokens stay blank so they are not replaced by accident."
            : "Choose a type, fill in what that cloud asks for, then create the remote. A browser sign-in stays open until rclone finishes."}
        </p>
        {loading && <p className="empty">Reading rclone backends…</p>}
        <form className="form" onSubmit={onSubmit}>
          <Field label="Name" hint="Letters, numbers, dashes, or underscores. Renaming copies the remote, then removes the old name.">
            <Input value={name} required placeholder="photos" onChange={(e) => setName(e.target.value)} />
          </Field>
          <Field label="Find a backend" hint="Filters the type list. LiClone loads every backend rclone has installed.">
            <Input value={query} placeholder="drive, s3, sftp…" onChange={(e) => setQuery(e.target.value)} />
          </Field>
          <Field
            label="Type"
            hint={
              provider?.description ||
              "Changing the type rebuilds the remote. Re-enter passwords and sign in again."
            }
          >
            <MenuSelect
              value={typeOptions.some((item) => item.value === kind) ? kind : ""}
              placeholder="Choose a backend"
              options={typeOptions}
              onChange={(next) => {
                setKind(next);
                if (!selected) setValues({});
              }}
            />
          </Field>
          <div className="field wide">
            <Button type="button" variant="ghost" size="sm" onClick={() => setAdvanced((on) => !on)}>
              {advanced ? "Hide unused fields" : "Add another field"}
            </Button>
          </div>
          {selected && (
            <section className="setting-block">
              <h2>Saved settings</h2>
              {!Object.keys(original).length && (
                <p className="help-note">
                  This remote has no extra settings stored, or rclone could not show them. Reconnect if it signs in with a browser.
                </p>
              )}
              {Object.keys(original).map((key) => {
                const option = provider?.options.find((item) => item.name === key);
                const secret = secretKey(key, option);
                return (
                  <Field
                    key={key}
                    label={key}
                    hint={
                      secret
                        ? "Already saved. Leave this blank to keep it, or type a new value to replace it."
                        : option?.help || "Stored in rclone’s config for this remote."
                    }
                  >
                    <Input
                      type={secret ? "password" : "text"}
                      value={values[key] ?? ""}
                      placeholder={secret ? "Saved" : undefined}
                      onChange={(e) => setField(key, e.target.value)}
                    />
                  </Field>
                );
              })}
            </section>
          )}
          {visibleOptions.map((option) => (
            <OptionField
              key={option.name}
              option={option}
              value={values[option.name] ?? ""}
              saved={Boolean(original[option.name])}
              onChange={(value) => setField(option.name, value)}
            />
          ))}
          {extras.map((key) => (
            <Field key={key} label={key} hint="Saved on this remote. Clear it and save to remove it.">
              <Input
                type={secretKey(key) ? "password" : "text"}
                value={values[key] ?? ""}
                placeholder={secretKey(key) && original[key] ? "Saved — type to replace" : undefined}
                onChange={(e) => setField(key, e.target.value)}
              />
            </Field>
          ))}
          <Field label="Add any other setting" hint="A config key rclone did not list. Example: acl private">
            <div className="path-row">
              <Input value={customKey} placeholder="key" onChange={(e) => setCustomKey(e.target.value)} />
              <Input value={customValue} placeholder="value" onChange={(e) => setCustomValue(e.target.value)} />
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  const key = customKey.trim();
                  if (!key) return;
                  setField(key, customValue);
                  setCustomKey("");
                  setCustomValue("");
                }}
              >
                Add
              </Button>
            </div>
          </Field>
          <div className="actions">
            <Button variant="primary" disabled={busy} type="submit">
              {busy ? "Saving…" : selected ? "Save remote" : "Create remote"}
            </Button>
            {selected && (
              <Button type="button" variant="ghost" disabled={busy} onClick={() => void reconnect()}>
                Reconnect
              </Button>
            )}
            {selected && (
              <Button type="button" variant="danger" disabled={busy} onClick={() => void remove()}>
                Delete
              </Button>
            )}
          </div>
        </form>
        {message && <p className="empty">{message}</p>}
      </div>
    </div>
  );
}

function OptionField({
  option,
  value,
  saved,
  onChange,
}: {
  option: ProviderOption;
  value: string;
  saved: boolean;
  onChange: (value: string) => void;
}) {
  const hint = [
    option.help,
    option.defaultValue ? `Default: ${option.defaultValue}` : "",
    option.required ? "Required for a new remote." : "",
    option.password && saved ? "A value is already saved. Type a new one only to replace it." : "",
  ]
    .filter(Boolean)
    .join(" ");
  const choices = option.examples.filter((example) => example.value);
  return (
    <Field label={option.name} hint={hint}>
      {choices.length > 0 && choices.length <= 12 && !option.password ? (
        <MenuSelect
          value={choices.some((example) => example.value === value) ? value : ""}
          placeholder={value || "Choose"}
          options={choices.map((example) => ({
            value: example.value,
            label: example.help ? `${example.value} — ${example.help}` : example.value,
          }))}
          onChange={onChange}
        />
      ) : null}
      <Input
        type={option.password ? "password" : "text"}
        value={value}
        placeholder={option.password && saved ? "Saved — type to replace" : option.defaultValue || option.type}
        onChange={(e) => onChange(e.target.value)}
      />
    </Field>
  );
}
