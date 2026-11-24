import { useId, useState, type KeyboardEvent, type ReactNode } from "react";
import { focusRing, type GridItemProps, type SettingEntry, type SettingValue } from "@om/shared";
import { KeyboardDialog } from "./KeyboardDialog";
import { APPLY_LABEL } from "./labels";

export const controlClass =
  "min-h-16 rounded-lg bg-surface-raised px-6 py-3 text-tv-md text-content " +
  `${focusRing} focus:bg-accent focus:text-accent-content ` +
  "aria-disabled:opacity-70";

export interface FieldProps {
  entry: SettingEntry;
  error?: string;
  onCommit: (value: SettingValue) => void;
  gridProps: GridItemProps;
}

interface ControlProps {
  id: string;
  labelId: string;
  describedBy: string;
  invalid: boolean;
  locked: boolean;
  entry: SettingEntry;
  onCommit: (value: SettingValue) => void;
  gridProps: GridItemProps;
}

/** Runs the control's own key handling first, then roving-grid navigation. */
function withGrid(
  grid: GridItemProps,
  own: (e: KeyboardEvent<HTMLElement>) => void,
): (e: KeyboardEvent<HTMLElement>) => void {
  return (e) => {
    own(e);
    grid.onKeyDown(e);
  };
}

function ToggleControl({ entry, onCommit, gridProps, locked, ...a }: ControlProps) {
  const checked = entry.value.type === "bool" && entry.value.value;
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-labelledby={a.labelId}
      aria-describedby={a.describedBy}
      aria-invalid={a.invalid || undefined}
      aria-disabled={locked || undefined}
      className={controlClass}
      onClick={() => !locked && onCommit({ type: "bool", value: !checked })}
      {...gridProps}
    >
      {checked ? "Activé" : "Désactivé"}
    </button>
  );
}

function StepperControl({ entry, onCommit, gridProps, locked, ...a }: ControlProps) {
  if (entry.descriptor.kind.type !== "int" || entry.value.type !== "int") return null;
  const { min, max, step } = entry.descriptor.kind;
  const value = entry.value.value;
  const unit = entry.descriptor.unit;
  // Out-of-range requests are sent anyway: Rust is the source of truth for bounds.
  const adjust = (delta: number) => !locked && onCommit({ type: "int", value: value + delta });
  const text = unit ? `${value} ${unit}` : String(value);
  return (
    <div
      role="spinbutton"
      aria-valuenow={value}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuetext={text}
      aria-labelledby={a.labelId}
      aria-describedby={a.describedBy}
      aria-invalid={a.invalid || undefined}
      aria-disabled={locked || undefined}
      className={controlClass}
      {...gridProps}
      onKeyDown={withGrid(gridProps, (e) => {
        if (e.key === "ArrowLeft") {
          e.preventDefault();
          adjust(-step);
        } else if (e.key === "ArrowRight") {
          e.preventDefault();
          adjust(step);
        }
      })}
    >
      <span aria-hidden="true">◀ </span>
      {text}
      <span aria-hidden="true"> ▶</span>
      <span className="ml-4 text-tv-sm text-content-muted">
        (de {min} à {max}, pas {step})
      </span>
    </div>
  );
}

function ChoiceControl({ entry, onCommit, gridProps, locked, ...a }: ControlProps) {
  if (entry.descriptor.kind.type !== "choice" || entry.value.type !== "choice") return null;
  const { options } = entry.descriptor.kind;
  const current = entry.value.value;
  const cycle = (delta: number) => {
    if (locked) return;
    const i = options.indexOf(current);
    onCommit({ type: "choice", value: options[(i + delta + options.length) % options.length] });
  };
  return (
    <button
      type="button"
      aria-labelledby={`${a.labelId} ${a.id}-value`}
      aria-describedby={a.describedBy}
      aria-invalid={a.invalid || undefined}
      aria-disabled={locked || undefined}
      className={controlClass}
      onClick={() => cycle(1)}
      {...gridProps}
      onKeyDown={withGrid(gridProps, (e) => {
        if (e.key === "ArrowLeft") {
          e.preventDefault();
          cycle(-1);
        } else if (e.key === "ArrowRight") {
          e.preventDefault();
          cycle(1);
        }
      })}
    >
      <span aria-hidden="true">◀ </span>
      <span id={`${a.id}-value`}>{current}</span>
      <span aria-hidden="true"> ▶</span>
    </button>
  );
}

function TextControl({ entry, onCommit, gridProps, locked, ...a }: ControlProps) {
  const [editing, setEditing] = useState(false);
  if (entry.descriptor.kind.type !== "text" || entry.value.type !== "text") return null;
  const { max_len } = entry.descriptor.kind;
  const current = entry.value.value;
  return (
    <>
      <button
        type="button"
        aria-labelledby={`${a.labelId} ${a.id}-value`}
        aria-describedby={a.describedBy}
        aria-invalid={a.invalid || undefined}
        aria-disabled={locked || undefined}
        aria-haspopup="dialog"
        className={controlClass}
        onClick={() => !locked && setEditing(true)}
        {...gridProps}
      >
        <span id={`${a.id}-value`}>{current === "" ? "Vide" : current}</span>
      </button>
      {editing && (
        <KeyboardDialog
          label={entry.descriptor.label}
          initialValue={current}
          maxLength={max_len}
          onCancel={() => setEditing(false)}
          onSubmit={(value) => {
            setEditing(false);
            onCommit({ type: "text", value });
          }}
        />
      )}
    </>
  );
}

const CONTROLS = {
  bool: ToggleControl,
  int: StepperControl,
  choice: ChoiceControl,
  text: TextControl,
} as const;

/** One generic row; the control is chosen from `descriptor.kind`, never from the key. */
export function SettingField({ entry, error, onCommit, gridProps }: FieldProps) {
  const id = useId();
  const { descriptor, locked_reason } = entry;
  const locked = locked_reason !== undefined;
  const labelId = `${id}-label`;
  const ids = [
    descriptor.description ? `${id}-desc` : null,
    `${id}-apply`,
    locked ? `${id}-lock` : null,
    error ? `${id}-error` : null,
  ].filter(Boolean);
  const Control = CONTROLS[descriptor.kind.type];
  let note: ReactNode = null;
  if (locked) {
    note = (
      <p id={`${id}-lock`} className="text-tv-sm text-content-muted">
        <span className="mr-2 rounded bg-content-muted px-2 text-surface">Verrouillé</span>
        {locked_reason}
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-2 py-4">
      <span id={labelId} className="text-tv-md">
        {descriptor.label}
      </span>
      {descriptor.description && (
        <p id={`${id}-desc`} className="text-tv-sm text-content-muted">
          {descriptor.description}
        </p>
      )}
      <Control
        id={id}
        labelId={labelId}
        describedBy={ids.join(" ")}
        invalid={error !== undefined}
        locked={locked}
        entry={entry}
        onCommit={onCommit}
        gridProps={gridProps}
      />
      <p id={`${id}-apply`} className="text-tv-sm text-content-muted">
        {APPLY_LABEL[descriptor.apply]}
      </p>
      {note}
      {error && (
        <p id={`${id}-error`} className="text-tv-sm text-danger">
          {error}
        </p>
      )}
    </div>
  );
}
