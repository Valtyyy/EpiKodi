import { useEffect, useRef, useState } from "react";
import { useRovingGrid, type AppError } from "@om/shared";
import { useSecretStatusUpdate, useSettings, useUpdateSettings } from "../api/settings";
import { formatError } from "../format";
import { controlClass } from "../settings/Field";
import { FIRST_RUN_KEY } from "./constants";
import { wizardSteps } from "./steps";

export interface FirstRunWizardProps {
  onDone: () => void;
  steps?: typeof wizardSteps;
}

export function FirstRunWizard({ onDone, steps = wizardSteps }: FirstRunWizardProps) {
  const secrets = useSettings().data?.secrets ?? [];
  const updateSettings = useUpdateSettings();
  const onSecretChange = useSecretStatusUpdate();
  const [index, setIndex] = useState(0);
  const [failure, setFailure] = useState<string>();
  const heading = useRef<HTMLHeadingElement>(null);
  const continueBtn = useRef<HTMLButtonElement>(null);

  const step = steps[index];
  const contentRows = step.rows;
  const grid = useRovingGrid({ rowLengths: [...contentRows, 2, 1] });
  const actionsRow = contentRows.length;
  const last = index === steps.length - 1;
  const progress = `Étape ${index + 1} sur ${steps.length}`;

  // The heading takes focus on each step so the screen reader reads step n of N and title.
  useEffect(() => {
    heading.current?.focus();
  }, [index]);

  useEffect(() => {
    if (failure) continueBtn.current?.focus();
  }, [failure]);

  async function complete() {
    try {
      await updateSettings.mutateAsync({
        changes: [{ key: FIRST_RUN_KEY, value: { type: "bool", value: true } }],
      });
      onDone();
    } catch (err) {
      setFailure(formatError(err as AppError));
    }
  }

  const next = () => (last ? void complete() : setIndex(index + 1));

  if (failure) {
    return (
      <main className="mx-auto max-w-4xl p-8">
        <h1 className="text-tv-xl">Assistant de démarrage</h1>
        <p className="mt-6 text-tv-md text-danger">{failure}</p>
        <p className="mt-4 text-tv-md text-content-muted">
          La fin de l'assistant n'a pas pu être enregistrée : il s'affichera de nouveau au prochain
          lancement. Vous pouvez continuer.
        </p>
        <button ref={continueBtn} type="button" className={`${controlClass} mt-6`} onClick={onDone}>
          Continuer
        </button>
      </main>
    );
  }

  return (
    <main className="mx-auto max-w-4xl p-8">
      <p className="text-tv-sm text-content-muted">{progress}</p>
      <h1
        ref={heading}
        tabIndex={-1}
        className="mb-4 text-tv-xl focus:outline-none"
        onKeyDown={(e) => {
          if (e.key === "ArrowDown") {
            e.preventDefault();
            grid.focusActive();
          }
        }}
      >
        <span className="sr-only">{progress} : </span>
        {step.title}
      </h1>
      <section aria-label={step.title} className="mb-8">
        <step.Component
          secrets={secrets}
          onSecretChange={onSecretChange}
          getGridProps={(row, col) => grid.getItemProps(row, col)}
        />
      </section>
      <div className="flex gap-3">
        <button
          type="button"
          className={controlClass}
          onClick={next}
          {...grid.getItemProps(actionsRow, 0)}
        >
          Passer cette étape
        </button>
        <button
          type="button"
          className={controlClass}
          onClick={next}
          {...grid.getItemProps(actionsRow, 1)}
        >
          {last ? "Terminer" : "Suivant"}
        </button>
      </div>
      <button
        type="button"
        className={`${controlClass} mt-6`}
        onClick={() => void complete()}
        {...grid.getItemProps(actionsRow + 1, 0)}
      >
        Passer l'assistant
      </button>
    </main>
  );
}
