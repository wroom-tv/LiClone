import { useState } from "react";
import { Button, Switch } from "./sealed/ui";
import { api } from "./api";
import type { RcloneInfo } from "./types";

export function Setup({
  info,
  installing,
  error: installError,
  onInstall,
  onDone,
}: {
  info: RcloneInfo | null;
  installing: boolean;
  error: string | null;
  onInstall: () => void;
  onDone: () => void;
}) {
  const [step, setStep] = useState(0);
  const [autostart, setAutostart] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function finish(start: boolean) {
    setBusy(true);
    setError(null);
    try {
      await api.completeSetup(start);
      onDone();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="setup-screen">
      <section className="setup-card">
        <img className="setup-mark" src="/images/Wroom_Logo_1Letter_Colored_1.png" alt="" />
        <p className="setup-step">Step {step + 1} of 3</p>
        {step === 0 && (
          <>
            <h2>Welcome to LiClone</h2>
            <p>
              LiClone is the desktop for your rclone mounts. It shows what is still on this PC and not on the cloud yet, and it keeps the mount settings in one place.
            </p>
          </>
        )}
        {step === 1 && (
          <>
            <h2>rclone</h2>
            {info?.found ? (
              <p>
                rclone is installed{info.version ? ` (${info.version})` : ""}.
                {info.compatible === false ? " LiClone needs rclone 1.65 or newer." : " LiClone can use it."}
              </p>
            ) : (
              <p>rclone is not on PATH yet. Install it now, or leave it and use the button in the header later.</p>
            )}
            {!info?.found && (
              <Button type="button" variant="primary" disabled={installing} onClick={onInstall}>
                {installing ? "Installing…" : "Install rclone"}
              </Button>
            )}
          </>
        )}
        {step === 2 && (
          <>
            <h2>Open with Windows</h2>
            <p>Recommended. LiClone starts when you sign in, so mounts and the upload list are ready without opening it by hand.</p>
            <Switch
              checked={autostart}
              label="Open LiClone when I sign in"
              description="Recommended. Adds a shortcut in the Windows Startup folder for this copy of LiClone."
              onChange={setAutostart}
            />
          </>
        )}
        {(error || (step === 1 && installError)) && <p className="setup-error">{error || installError}</p>}
        <div className="setup-actions">
          {step > 0 ? (
            <Button type="button" variant="ghost" disabled={busy} onClick={() => setStep((n) => n - 1)}>
              Back
            </Button>
          ) : (
            <Button type="button" variant="ghost" disabled={busy} onClick={() => void finish(false)}>
              Skip
            </Button>
          )}
          {step < 2 ? (
            <Button type="button" variant="primary" onClick={() => setStep((n) => n + 1)}>
              Continue
            </Button>
          ) : (
            <Button type="button" variant="primary" disabled={busy} onClick={() => void finish(autostart)}>
              {busy ? "Saving…" : "Finish"}
            </Button>
          )}
        </div>
      </section>
    </div>
  );
}
