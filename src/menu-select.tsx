import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

export function MenuSelect({
  value,
  options,
  placeholder,
  onChange,
}: {
  value: string;
  options: { value: string; label: string }[];
  placeholder?: string;
  onChange: (value: string) => void;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [box, setBox] = useState({ top: 0, left: 0, width: 0 });
  const current = options.find((option) => option.value === value);

  function place() {
    const rect = rootRef.current?.getBoundingClientRect();
    if (!rect) return;
    const width = Math.max(rect.width, 220);
    const left = Math.min(rect.left, window.innerWidth - width - 12);
    setBox({ top: rect.bottom + 6, left: Math.max(12, left), width });
  }

  useEffect(() => {
    if (!open) return;
    place();
    const onScroll = () => place();
    const onDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (rootRef.current?.contains(target) || menuRef.current?.contains(target)) return;
      setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onScroll);
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onScroll);
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="pick" ref={rootRef}>
      <button
        type="button"
        className="pick-trigger"
        aria-expanded={open}
        onClick={() => {
          if (open) setOpen(false);
          else {
            place();
            setOpen(true);
          }
        }}
      >
        <span>{current?.label || placeholder || "Choose"}</span>
        <i />
      </button>
      {open &&
        createPortal(
          <div ref={menuRef} className="pick-menu" style={{ top: box.top, left: box.left, width: box.width }}>
            {!options.length && <p>Nothing to choose yet.</p>}
            {options.map((option) => (
              <button
                type="button"
                key={option.value || option.label}
                className={option.value === value ? "on" : ""}
                onClick={() => {
                  onChange(option.value);
                  setOpen(false);
                }}
              >
                {option.label}
              </button>
            ))}
          </div>,
          document.body,
        )}
    </div>
  );
}
