import type { GridItemProps, SecretStatus } from "@om/shared";
import { SecretField } from "../settings/SecretField";

export interface StepProps {
  secrets: SecretStatus[];
  onSecretChange: (status: SecretStatus) => void;
  /** Roving-grid props for focusable items of this step: (local row, column). */
  getGridProps: (row: number, col: number) => GridItemProps;
}

export function SourceStep() {
  return (
    <p className="text-tv-md text-content-muted">
      Les sources (dossiers locaux ou partages réseau) seront ajoutées depuis la gestion des
      sources. Vous pourrez le faire plus tard, cette étape peut être passée.
    </p>
  );
}

export function TmdbStep({ secrets, onSecretChange, getGridProps }: StepProps) {
  const status = secrets.find((s) => s.key === "tmdb_api_key");
  return (
    <>
      <p className="text-tv-md text-content-muted">
        Une clé TMDB permet de récupérer affiches et descriptions. Sans elle, la bibliothèque
        fonctionne avec les seuls noms de fichiers.
      </p>
      {status && (
        <SecretField
          status={status}
          onStatusChange={onSecretChange}
          getGridProps={(col) => getGridProps(0, col)}
        />
      )}
    </>
  );
}

export function RemoteStep() {
  return (
    <p className="text-tv-md text-content-muted">
      L'adresse de la télécommande et le jumelage du téléphone arriveront avec la télécommande web.
      Cette étape peut être passée.
    </p>
  );
}
