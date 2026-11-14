import type { ComponentType } from "react";
import { RemoteStep, SourceStep, TmdbStep, type StepProps } from "./stepComponents";

export interface WizardStep {
  id: string;
  title: string;
  description: string;
  /** Number of focusable items in each row of the step's own content (excluding the wizard actions). */
  rows: number[];
  Component: ComponentType<StepProps>;
}

// Why a registry: EPIC 05 (#43, source form) and EPIC 09 (#84, remote address) replace the
// placeholder `Component` of their step without touching the wizard's navigation or completion.
export const wizardSteps: WizardStep[] = [
  {
    id: "source",
    title: "Ajouter une source",
    description: "Où se trouvent vos médias",
    rows: [],
    Component: SourceStep,
  },
  {
    id: "tmdb",
    title: "Clé TMDB (facultatif)",
    description: "Affiches et descriptions",
    rows: [2],
    Component: TmdbStep,
  },
  {
    id: "remote",
    title: "Adresse de la télécommande",
    description: "Piloter depuis un téléphone",
    rows: [],
    Component: RemoteStep,
  },
];
