import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { VirtualKeyboard } from "../keyboard/VirtualKeyboard";

export interface KeyboardDialogProps {
  label: string;
  initialValue?: string;
  secret?: boolean;
  maxLength?: number;
  onSubmit: (value: string) => void;
  onCancel: () => void;
}

/** Full-screen modal around the virtual keyboard; gives focus back to the opener on close. */
export function KeyboardDialog({
  label,
  initialValue = "",
  secret,
  maxLength,
  onSubmit,
  onCancel,
}: KeyboardDialogProps) {
  const [draft, setDraft] = useState(initialValue);
  const opener = useRef(document.activeElement);

  useEffect(() => {
    const el = opener.current;
    return () => {
      if (el instanceof HTMLElement) el.focus();
    };
  }, []);

  // Navigation is arrow-driven; Tab must not leave the modal.
  const trapTab = (e: KeyboardEvent) => {
    if (e.key === "Tab") e.preventDefault();
  };

  function submit() {
    const value = draft;
    // The draft may be a secret: drop it from state before handing it over.
    setDraft("");
    onSubmit(value);
  }

  return createPortal(
    <div
      role="dialog"
      aria-modal="true"
      aria-label={label}
      onKeyDown={trapTab}
      className="fixed inset-0 z-50 flex items-center justify-center bg-surface"
    >
      <div className="w-full max-w-5xl">
        <VirtualKeyboard
          label={label}
          value={draft}
          onChange={setDraft}
          onSubmit={submit}
          onCancel={onCancel}
          secret={secret}
          maxLength={maxLength}
        />
      </div>
    </div>,
    document.body,
  );
}
