import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  ApiProvider,
  createFakeApiClient,
  type InputAction,
  type SettingsSnapshot,
} from "@om/shared";
import { describe, expect, it, vi } from "vitest";
import { SettingsScreen } from "../settings/SettingsScreen";
import { TvInput } from "./TvInput";

const snapshot: SettingsSnapshot = {
  warnings: [],
  secrets: [],
  entries: [
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
    {
      source: "default",
      value: { type: "int", value: 30000 },
      descriptor: {
        key: "network.timeout_ms",
        section: "Réseau",
        label: "Délai réseau",
        kind: { type: "int", min: 29000, max: 31000, step: 1000 },
        default: { type: "int", value: 30000 },
        apply: "next_operation",
        visible: true,
      },
    },
  ],
};

function setup() {
  const client = createFakeApiClient(snapshot);
  render(
    <ApiProvider client={client}>
      <TvInput>
        <SettingsScreen />
      </TvInput>
    </ApiProvider>,
  );
  const remote = (action: InputAction) => act(() => client._emitUiEvent({ type: "input", action }));
  return { remote, user: userEvent.setup() };
}

/** Same scenario, driven once by the keyboard and once by the bridge. */
const scenarios = {
  keyboard: (s: ReturnType<typeof setup>) => ({
    down: () => s.user.keyboard("{ArrowDown}"),
    right: () => s.user.keyboard("{ArrowRight}"),
    up: () => s.user.keyboard("{ArrowUp}"),
    ok: () => s.user.keyboard("{Enter}"),
  }),
  bridge: (s: ReturnType<typeof setup>) => ({
    down: () => s.remote("down"),
    right: () => s.remote("right"),
    up: () => s.remote("up"),
    ok: () => s.remote("ok"),
  }),
};

describe("TvInput", () => {
  for (const [source, drive] of Object.entries(scenarios)) {
    it(`gives the same result from the ${source}`, async () => {
      const input = drive(setup());
      const toggle = await screen.findByRole("switch", { name: "Barre de titre" });
      await waitFor(() => expect(document.activeElement).toBe(toggle));

      await input.ok();
      await waitFor(() => expect(toggle).toHaveAttribute("aria-checked", "false"));

      await input.down();
      const spin = screen.getByRole("spinbutton", { name: "Délai réseau" });
      expect(document.activeElement).toBe(spin);
      await input.right();
      await waitFor(() => expect(spin).toHaveAttribute("aria-valuenow", "31000"));

      await input.up();
      expect(document.activeElement).toBe(toggle);
    });
  }

  it("clicks a button once on Enter (native activation is replaced, not doubled)", async () => {
    const onClick = vi.fn();
    render(
      <ApiProvider client={createFakeApiClient(snapshot)}>
        <TvInput>
          <button type="button" onClick={onClick}>
            Lire
          </button>
        </TvInput>
      </ApiProvider>,
    );
    screen.getByRole("button", { name: "Lire" }).focus();
    await userEvent.setup().keyboard("{Enter}");
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
