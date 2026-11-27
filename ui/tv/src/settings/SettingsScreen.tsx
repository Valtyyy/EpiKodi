import { useEffect, useState } from "react";
import { useRovingGrid, type AppError, type SettingValue } from "@om/shared";
import { useSecretStatusUpdate, useSettings, useUpdateSettings } from "../api/settings";
import { formatError } from "../format";
import { SettingField } from "./Field";
import { SecretField } from "./SecretField";

const SECRETS_SECTION = "Métadonnées";

export function SettingsScreen() {
  const settings = useSettings();
  const updateSettings = useUpdateSettings();
  const onSecretChange = useSecretStatusUpdate();
  const snapshot = settings.data;
  const loadError = settings.error ? formatError(settings.error) : undefined;
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [announcement, setAnnouncement] = useState("");

  const visible = snapshot?.entries.filter((e) => e.descriptor.visible) ?? [];
  const secrets = snapshot?.secrets ?? [];
  const grid = useRovingGrid({
    rowLengths: [...visible.map(() => 1), ...secrets.map(() => 2)],
  });
  const { focusActive } = grid;
  const loaded = snapshot !== undefined;

  useEffect(() => {
    if (loaded) focusActive();
    // Only the first load should grab focus, not every later focus move.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loaded]);

  async function commit(key: string, label: string, value: SettingValue) {
    try {
      await updateSettings.mutateAsync({ changes: [{ key, value }] });
      setErrors((e) => ({ ...e, [key]: "" }));
      setAnnouncement(`${label} enregistré`);
    } catch (err) {
      const e = err as AppError;
      const msg = formatError(e);
      setErrors((prev) => ({ ...prev, [e.field ?? key]: msg }));
      setAnnouncement(msg);
    }
  }

  const sections: { name: string; entries: typeof visible }[] = [];
  for (const entry of visible) {
    const name = entry.descriptor.section;
    const group = sections.find((s) => s.name === name);
    if (group) group.entries.push(entry);
    else sections.push({ name, entries: [entry] });
  }

  let row = 0;
  return (
    <main className="mx-auto max-w-5xl p-8">
      <h1 className="mb-6 text-tv-xl">Réglages</h1>
      {loadError && <p className="text-danger">{loadError}</p>}
      {snapshot && snapshot.warnings.length > 0 && (
        <ul aria-label="Avertissements" className="mb-6 rounded-lg bg-surface-raised p-4">
          {snapshot.warnings.map((w) => (
            <li key={w.key + w.message} className="text-tv-sm text-danger">
              {w.message}
            </li>
          ))}
        </ul>
      )}
      {sections.map((section) => (
        <section key={section.name} aria-label={section.name} className="mb-8">
          <h2 className="border-b border-content-muted pb-2 text-tv-lg">{section.name}</h2>
          {section.entries.map((entry) => {
            const r = row++;
            const key = entry.descriptor.key;
            return (
              <SettingField
                key={key}
                entry={entry}
                error={errors[key] || undefined}
                gridProps={grid.getItemProps(r, 0)}
                onCommit={(value) => void commit(key, entry.descriptor.label, value)}
              />
            );
          })}
        </section>
      ))}
      {secrets.length > 0 && (
        <section aria-label={SECRETS_SECTION} className="mb-8">
          <h2 className="border-b border-content-muted pb-2 text-tv-lg">{SECRETS_SECTION}</h2>
          {secrets.map((status) => {
            const r = row++;
            return (
              <SecretField
                key={status.key}
                status={status}
                onStatusChange={onSecretChange}
                getGridProps={(col) => grid.getItemProps(r, col)}
              />
            );
          })}
        </section>
      )}
      {/* aria-live may not reach Orca under WebKitGTK (spike 2.5); an Announcer port adds a fallback. */}
      <p role="status" aria-live="polite" aria-label="Annonces" className="sr-only">
        {announcement}
      </p>
    </main>
  );
}
