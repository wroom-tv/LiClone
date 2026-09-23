import { useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

function windowOrNull() {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
}

export function TitleBar() {
  const [maxed, setMaxed] = useState(false);

  function act(fn: (win: ReturnType<typeof getCurrentWindow>) => void) {
    const win = windowOrNull();
    if (win) fn(win);
  }

  return (
    <header className="titlebar">
      <div className="titlebar-drag" data-tauri-drag-region>
        <div className="brand-lockup" data-tauri-drag-region>
          <img
            className="brand-logo"
            src="/images/Wroom_Logo_Full_Colored_1.png"
            alt="Wroom"
            draggable={false}
          />
          <span className="brand-product">LiClone</span>
        </div>
      </div>
      <div className="win-controls">
        <button type="button" aria-label="Minimize" onClick={() => act((win) => void win.minimize())}>
          <svg viewBox="0 0 12 12">
            <path d="M2 6h8" />
          </svg>
        </button>
        <button
          type="button"
          aria-label={maxed ? "Restore" : "Maximize"}
          onClick={() =>
            act((win) => {
              void win.toggleMaximize().then(() => win.isMaximized().then(setMaxed).catch(() => {}));
            })
          }
        >
          {maxed ? (
            <svg viewBox="0 0 12 12">
              <path d="M4 3h5v5M3 4h5v5" />
            </svg>
          ) : (
            <svg viewBox="0 0 12 12">
              <rect x="2.5" y="2.5" width="7" height="7" rx="1" />
            </svg>
          )}
        </button>
        <button type="button" className="close" aria-label="Close" onClick={() => act((win) => void win.close())}>
          <svg viewBox="0 0 12 12">
            <path d="M3 3l6 6M9 3L3 9" />
          </svg>
        </button>
      </div>
    </header>
  );
}
