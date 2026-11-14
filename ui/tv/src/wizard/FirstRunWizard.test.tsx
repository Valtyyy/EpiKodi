import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createFakeApiClient, type SettingsSnapshot } from "@om/shared";
import { describe, expect, it, vi } from "vitest";
import App from "../App";
import { axeViolations } from "../test/axe";
import { FIRST_RUN_KEY } from "./constants";

function snapshot(firstRunCompleted: boolean): SettingsSnapshot {
  return {
    entries: [
      {
        source: "default",
        value: { type: "bool", value: firstRunCompleted },
        descriptor: {
          key: FIRST_RUN_KEY,
          section: "Configuration",
          label: "Assistant terminé",
          kind: { type: "bool" },
          default: { type: "bool", value: false },
          apply: "immediate",
          visible: false,
        },
      },
      {
        source: "default",
        value: { type: "bool", value: true },
        descriptor: {
          key: "interface.window_decorations",
          section: "Interface",
          label: "Barre de titre",
          kind: { type: "bool" },
          default: { type: "bool", value: true },
          apply: "immediate",
          visible: true,
        },
      },
    ],
    secrets: [{ key: "tmdb_api_key", configured: false, source: "none", state: "ok" }],
    warnings: [],
  };
}

const completed = (client: ReturnType<typeof createFakeApiClient>) =>
  client
    .getSettings()
    .then((s) => s.entries.find((e) => e.descriptor.key === FIRST_RUN_KEY)?.value);

function setup(done = false) {
  const client = createFakeApiClient(snapshot(done));
  const user = userEvent.setup();
  const utils = render(<App client={client} />);
  return { client, user, ...utils };
}

const settingsHeading = () => screen.findByRole("heading", { name: "Réglages" });
const stepHeading = (name: RegExp | string) => screen.findByRole("heading", { name });

describe("first-run wizard", () => {
  it("is shown when first_run_completed is false", async () => {
    setup(false);
    expect(await stepHeading(/Ajouter une source/)).toBeInTheDocument();
    expect(screen.getByText("Étape 1 sur 3")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Réglages" })).toBeNull();
  });

  it("is not shown when first_run_completed is true", async () => {
    setup(true);
    expect(await settingsHeading()).toBeInTheDocument();
    expect(screen.queryByText(/Étape 1 sur 3/)).toBeNull();
  });

  it("moves focus to the step heading on each step", async () => {
    const { user } = setup();
    const h1 = await stepHeading(/Ajouter une source/);
    await waitFor(() => expect(h1).toHaveFocus());
    await user.click(screen.getByRole("button", { name: "Suivant" }));
    const h2 = await stepHeading(/Clé TMDB/);
    expect(h2).toHaveFocus();
    expect(h2).toHaveTextContent("Étape 2 sur 3");
  });

  it("lets every step be skipped, then finishes and shows settings", async () => {
    const { user, client } = setup();
    await stepHeading(/Ajouter une source/);
    await user.click(screen.getByRole("button", { name: "Passer cette étape" }));
    await stepHeading(/Clé TMDB/);
    await user.click(screen.getByRole("button", { name: "Passer cette étape" }));
    await stepHeading(/Adresse de la télécommande/);
    expect(screen.queryByRole("button", { name: "Suivant" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Passer cette étape" }));
    expect(await settingsHeading()).toBeInTheDocument();
    expect(await completed(client)).toEqual({ type: "bool", value: true });
  });

  it("Terminer on the last step marks completed", async () => {
    const { user, client } = setup();
    await stepHeading(/Ajouter/);
    await user.click(screen.getByRole("button", { name: "Suivant" }));
    await user.click(await screen.findByRole("button", { name: "Suivant" }));
    await user.click(await screen.findByRole("button", { name: "Terminer" }));
    expect(await settingsHeading()).toBeInTheDocument();
    expect(await completed(client)).toEqual({ type: "bool", value: true });
  });

  it("Passer l'assistant marks completed from any step", async () => {
    const { user, client } = setup();
    await stepHeading(/Ajouter/);
    await user.click(screen.getByRole("button", { name: "Passer l'assistant" }));
    expect(await settingsHeading()).toBeInTheDocument();
    expect(await completed(client)).toEqual({ type: "bool", value: true });
  });

  it("navigates between actions with the arrows", async () => {
    const { user } = setup();
    await stepHeading(/Ajouter/);
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("button", { name: "Passer cette étape" })).toHaveFocus();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("button", { name: "Suivant" })).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("button", { name: "Passer l'assistant" })).toHaveFocus();
  });

  it("sets the TMDB key without ever rendering it", async () => {
    const SECRET = "0123456789abcdef0123456789abcdef";
    const { user, client } = setup();
    await stepHeading(/Ajouter/);
    await user.click(screen.getByRole("button", { name: "Suivant" }));
    await user.click(await screen.findByRole("button", { name: "Saisir Clé API TMDB" }));
    const dialog = await screen.findByRole("dialog");
    await user.keyboard(SECRET);
    await user.click(within(dialog).getByRole("button", { name: "Valider" }));
    await waitFor(() => expect(screen.getByText("Configurée")).toBeInTheDocument());
    expect(client._secretValue("tmdb_api_key")).toBe(SECRET);
    expect(document.body.innerHTML).not.toContain(SECRET);
    // Moving on to the next step still never shows the value.
    await user.click(screen.getByRole("button", { name: "Suivant" }));
    expect(document.body.innerHTML).not.toContain(SECRET);
  });

  it("shows the error but lets the user continue when completion fails", async () => {
    const { user, client } = setup();
    vi.spyOn(client, "updateSettings").mockRejectedValue({ code: "io", message: "Disque plein" });
    await stepHeading(/Ajouter/);
    await user.click(screen.getByRole("button", { name: "Passer l'assistant" }));
    expect(await screen.findByText("Disque plein")).toBeInTheDocument();
    expect(screen.getByText(/s'affichera de nouveau au prochain lancement/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Continuer" }));
    expect(await settingsHeading()).toBeInTheDocument();
  });

  it("has no axe violations on every step", async () => {
    const { user, container } = setup();
    await stepHeading(/Ajouter/);
    expect(await axeViolations(container)).toEqual([]);
    await user.click(screen.getByRole("button", { name: "Suivant" }));
    await stepHeading(/Clé TMDB/);
    expect(await axeViolations(container)).toEqual([]);
    await user.click(screen.getByRole("button", { name: "Suivant" }));
    await stepHeading(/Adresse/);
    expect(await axeViolations(container)).toEqual([]);
  });
});
