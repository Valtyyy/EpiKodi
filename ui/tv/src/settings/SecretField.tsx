import { useId, useState } from "react";
import { useApi, type AppError, type GridItemProps, type SecretStatus } from "@om/shared";
import { formatError } from "../format";
import { controlClass } from "./Field";
import { KeyboardDialog } from "./KeyboardDialog";
import { secretLabel, secretStateText } from "./labels";

export interface SecretFieldProps {
  status: SecretStatus;
  onStatusChange: (status: SecretStatus) => void;
  /** Roving-grid props for the two action buttons (Remplacer/Saisir, Effacer). */
  getGridProps?: (col: number) => GridItemProps;
}

/**
 * Write-only secret editor (C7): shows state text only. The typed value lives in the
 * keyboard dialog's state, is dropped on submit and is never rendered back.
 */
export function SecretField({ status, onStatusChange, getGridProps }: SecretFieldProps) {
  const api = useApi();
  const id = useId();
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState<string>();
  const [announcement, setAnnouncement] = useState("");
  const label = secretLabel(status.key);
  const writable = status.state === "ok";

  async function run(action: () => Promise<SecretStatus>, done: (s: SecretStatus) => string) {
    try {
      const next = await action();
      setError(undefined);
      onStatusChange(next);
      setAnnouncement(done(next));
    } catch (err) {
      const msg = formatError(err as AppError);
      setError(msg);
      setAnnouncement(msg);
    }
  }

  const describedBy = `${id}-state${error ? ` ${id}-error` : ""}`;
  return (
    <div className="flex flex-col gap-2 py-4">
      <span id={`${id}-label`} className="text-tv-md">
        {label}
      </span>
      <p id={`${id}-state`} className="text-tv-md">
        {secretStateText(status)}
      </p>
      <div className="flex gap-3">
        <button
          type="button"
          aria-label={`${status.configured ? "Remplacer" : "Saisir"} ${label}`}
          aria-describedby={describedBy}
          aria-disabled={!writable || undefined}
          aria-haspopup="dialog"
          className={controlClass}
          onClick={() => writable && setEditing(true)}
          {...getGridProps?.(0)}
        >
          {status.configured ? "Remplacer" : "Saisir"}
        </button>
        <button
          type="button"
          aria-label={`Effacer ${label}`}
          aria-describedby={describedBy}
          aria-disabled={!writable || !status.configured || undefined}
          className={controlClass}
          onClick={() =>
            writable &&
            status.configured &&
            void run(
              () => api.clearSecret(status.key),
              () => `${label} effacée`,
            )
          }
          {...getGridProps?.(1)}
        >
          Effacer
        </button>
      </div>
      {error && (
        <p id={`${id}-error`} className="text-tv-sm text-danger">
          {error}
        </p>
      )}
      {/* The announcement never contains the value, only the outcome. */}
      <p role="status" aria-live="polite" aria-label={`Annonce ${label}`} className="sr-only">
        {announcement}
      </p>
      {editing && (
        <KeyboardDialog
          label={label}
          secret
          onCancel={() => setEditing(false)}
          onSubmit={(value) => {
            setEditing(false);
            void run(
              () => api.setSecret(status.key, value),
              () => `${label} enregistrée`,
            );
          }}
        />
      )}
    </div>
  );
}
