// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SettingsSnapshot, UiEvent } from "../index";
import type { ApiClient } from "./client";
import { ApiProvider, useTvApi } from "./context";
import { createFakeApiClient } from "./fake";

afterEach(cleanup);

const empty: SettingsSnapshot = { entries: [], secrets: [], warnings: [] };

function TvOnly() {
  useTvApi();
  return <p>tv</p>;
}

describe("ApiProvider", () => {
  it("gives TV components the TV client", () => {
    render(
      <ApiProvider client={createFakeApiClient(empty)}>
        <TvOnly />
      </ApiProvider>,
    );
    expect(screen.getByText("tv")).toBeTruthy();
  });

  it("refuses a TV-only component under the remote's client", () => {
    const remote: ApiClient = { sendInput: async () => {} };
    vi.spyOn(console, "error").mockImplementation(() => {});
    expect(() =>
      render(
        <ApiProvider client={remote}>
          <TvOnly />
        </ApiProvider>,
      ),
    ).toThrow(/TvApiClient/);
  });
});

describe("fake client", () => {
  it("loops sendInput back as a UiEvent, like InputService and the UiBridge", async () => {
    const client = createFakeApiClient(empty);
    const events: UiEvent[] = [];
    const stop = client.onUiEvent((e) => events.push(e));
    await client.sendInput("left");
    stop();
    await client.sendInput("right");
    expect(events).toEqual([{ type: "input", action: "left" }]);
  });
});
