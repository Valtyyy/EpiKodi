import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  ApiProvider,
  createFakeApiClient,
  type SettingEntry,
  type SettingsSnapshot,
} from "@om/shared";
import { describe, expect, it } from "vitest";
import { axeViolations } from "../test/axe";
import { SettingsScreen } from "./SettingsScreen";

const base = {
  source: "default",
  value: { type: "bool", value: true },
} as const;

const entries: SettingEntry[] = [
  {
    ...base,
    descriptor: {
      key: "interface.window_decorations",
      section: "Interface",
      label: "Barre de titre",
      description: "Affiche la barre de titre de la fenêtre",
      kind: { type: "bool" },
      default: { type: "bool", value: true },
      apply: "immediate",
      visible: true,
    },
  },
  {
    source: "default",
    value: { type: "int", value: 30000 },
    descriptor: {
      key: "network.timeout_ms",
      section: "Réseau",
      label: "Délai réseau",
      kind: { type: "int", min: 29000, max: 31000, step: 1000 },
      default: { type: "int", value: 30000 },
      unit: "ms",
      apply: "next_operation",
      visible: true,
    },
  },
  {
    source: "env",
    value: { type: "choice", value: "info" },
    locked_reason: "Imposé par OPEN_MOSAIK_LOG_LEVEL",
    descriptor: {
      key: "log.level",
      section: "Interface",
      label: "Niveau de journal",
      kind: { type: "choice", options: ["info", "debug"] },
      default: { type: "choice", value: "info" },
      apply: "next_playback",
      visible: true,
    },
  },
  {
    ...base,
    value: { type: "bool", value: false },
    descriptor: {
      key: "internal.hidden",
      section: "Interne",
      label: "Réglage caché",
      kind: { type: "bool" },
      default: { type: "bool", value: false },
      apply: "immediate",
      visible: false,
    },
  },
];

const snapshot = (over: Partial<SettingsSnapshot> = {}): SettingsSnapshot => ({
  entries,
  secrets: [{ key: "tmdb_api_key", configured: false, source: "none", state: "ok" }],
  warnings: [{ key: "network.timeout_ms", message: "Valeur de repli utilisée" }],
  ...over,
});

function setup(snap = snapshot()) {
  const client = createFakeApiClient(snap);
  const user = userEvent.setup();
  const utils = render(
    <ApiProvider client={client}>
      <SettingsScreen />
    </ApiProvider>,
  );
  return { client, user, ...utils };
}

describe("SettingsScreen", () => {
  it("renders visible entries by section with apply moment and warnings", async () => {
    setup();
    expect(await screen.findByRole("switch", { name: "Barre de titre" })).toBeChecked();
    expect(screen.getByRole("heading", { name: "Réseau" })).toBeInTheDocument();
    expect(screen.getByText("Appliqué immédiatement")).toBeInTheDocument();
    expect(screen.getByText("À la prochaine opération")).toBeInTheDocument();
    expect(screen.getByText("Valeur de repli utilisée")).toBeInTheDocument();
    expect(screen.queryByText("Réglage caché")).toBeNull();
  });

  it("renders a field for a newly added descriptor with no new component code", async () => {
    const extra: SettingEntry = {
      source: "default",
      value: { type: "text", value: "salon" },
      descriptor: {
        key: "interface.room",
        section: "Interface",
        label: "Nom de la pièce",
        kind: { type: "text", max_len: 20 },
        default: { type: "text", value: "salon" },
        apply: "immediate",
        visible: true,
      },
    };
    setup(snapshot({ entries: [...entries, extra] }));
    expect(await screen.findByRole("button", { name: /Nom de la pièce/ })).toBeInTheDocument();
  });

  it("toggles a bool via keyboard and persists through the client", async () => {
    const { user } = setup();
    const sw = await screen.findByRole("switch", { name: "Barre de titre" });
    sw.focus();
    await user.keyboard("{Enter}");
    await waitFor(() =>
      expect(screen.getByRole("switch", { name: "Barre de titre" })).not.toBeChecked(),
    );
  });

  it("shows locked badge and reason and refuses changes", async () => {
    const { user } = setup();
    const control = await screen.findByRole("button", { name: /Niveau de journal/ });
    expect(control).toHaveAttribute("aria-disabled", "true");
    expect(control).toHaveAccessibleDescription(/Imposé par OPEN_MOSAIK_LOG_LEVEL/);
    expect(screen.getByText("Verrouillé")).toBeInTheDocument();
    control.focus();
    await user.keyboard("{ArrowRight}{Enter}");
    expect(control).toHaveTextContent("info");
  });

  it("steps numbers and shows the Rust-side bound error on the field", async () => {
    const { user } = setup();
    const spin = await screen.findByRole("spinbutton", { name: "Délai réseau" });
    expect(spin).toHaveAttribute("aria-valuemin", "29000");
    spin.focus();
    await user.keyboard("{ArrowRight}");
    await waitFor(() => expect(spin).toHaveAttribute("aria-valuenow", "31000"));
    await user.keyboard("{ArrowRight}");
    await waitFor(() => expect(spin).toHaveAttribute("aria-invalid", "true"));
    expect(spin).toHaveAttribute("aria-valuenow", "31000");
    expect(spin).toHaveAccessibleDescription(/comprise entre 29000 et 31000/);
    // A successful change clears the error.
    await user.keyboard("{ArrowLeft}");
    await waitFor(() => expect(spin).not.toHaveAttribute("aria-invalid"));
  });

  it("moves focus between fields with Up/Down", async () => {
    const { user } = setup();
    const sw = await screen.findByRole("switch", { name: "Barre de titre" });
    sw.focus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("button", { name: /Niveau de journal/ })).toHaveFocus();
  });

  it("edits a secret without ever rendering it, then clears it", async () => {
    const SECRET = "deadbeefcafe0123456789abcdef0123";
    const { user, client } = setup();
    const set = await screen.findByRole("button", { name: "Saisir Clé API TMDB" });
    expect(screen.getByText("Non configurée")).toBeInTheDocument();
    set.focus();
    await user.keyboard("{Enter}");
    const dialog = await screen.findByRole("dialog", { name: "Clé API TMDB" });
    await user.keyboard(SECRET);
    expect(document.body.innerHTML).not.toContain(SECRET);
    await user.click(within(dialog).getByRole("button", { name: "Valider" }));
    await waitFor(() => expect(screen.getByText("Configurée")).toBeInTheDocument());
    expect(client._secretValue("tmdb_api_key")).toBe(SECRET);
    expect(document.body.innerHTML).not.toContain(SECRET);
    expect(document.body.textContent).not.toContain(SECRET);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("status", { name: "Annonce Clé API TMDB" })).toHaveTextContent(
      "Clé API TMDB enregistrée",
    );
    // Focus returns to the opener.
    expect(screen.getByRole("button", { name: "Remplacer Clé API TMDB" })).toHaveFocus();

    await user.click(screen.getByRole("button", { name: "Effacer Clé API TMDB" }));
    await waitFor(() => expect(screen.getByText("Non configurée")).toBeInTheDocument());
    expect(client._secretValue("tmdb_api_key")).toBeUndefined();
  });

  it("shows env-locked secrets as read-only", async () => {
    setup(
      snapshot({
        secrets: [{ key: "tmdb_api_key", configured: true, source: "env", state: "env_locked" }],
      }),
    );
    expect(
      await screen.findByText("Imposée par la variable OPEN_MOSAIK_TMDB_API_KEY"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Remplacer Clé API TMDB" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  });

  it("has no axe violations", async () => {
    const { container } = setup();
    await screen.findByRole("switch", { name: "Barre de titre" });
    expect(await axeViolations(container)).toEqual([]);
  });
});
