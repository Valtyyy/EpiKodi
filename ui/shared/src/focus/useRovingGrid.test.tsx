// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { useRovingGrid } from "./useRovingGrid";

afterEach(cleanup);

function Grid({ rows }: { rows: number[] }) {
  const grid = useRovingGrid({ rowLengths: rows });
  return (
    <div>
      {rows.map((len, r) =>
        Array.from({ length: len }, (_, c) => (
          <button key={`${r}-${c}`} {...grid.getItemProps(r, c)}>
            {`${r}.${c}`}
          </button>
        )),
      )}
    </div>
  );
}

const press = (key: string) => fireEvent.keyDown(document.activeElement!, { key });

describe("useRovingGrid", () => {
  it("makes only the active item tabbable", () => {
    render(<Grid rows={[3, 2]} />);
    const tabbable = screen.getAllByRole("button").filter((b) => b.tabIndex === 0);
    expect(tabbable.map((b) => b.textContent)).toEqual(["0.0"]);
  });

  it("moves real DOM focus with the arrows and keeps roving tabindex in sync", () => {
    render(<Grid rows={[3, 3]} />);
    screen.getByText("0.0").focus();
    press("ArrowRight");
    expect(document.activeElement).toBe(screen.getByText("0.1"));
    press("ArrowDown");
    expect(document.activeElement).toBe(screen.getByText("1.1"));
    press("ArrowLeft");
    press("ArrowUp");
    expect(document.activeElement).toBe(screen.getByText("0.0"));
    expect(screen.getByText("0.0").tabIndex).toBe(0);
    expect(screen.getByText("1.1").tabIndex).toBe(-1);
  });

  it("does not wrap at the edges", () => {
    render(<Grid rows={[2, 2]} />);
    screen.getByText("0.0").focus();
    press("ArrowLeft");
    press("ArrowUp");
    expect(document.activeElement).toBe(screen.getByText("0.0"));
    screen.getByText("1.1").focus();
    press("ArrowRight");
    press("ArrowDown");
    expect(document.activeElement).toBe(screen.getByText("1.1"));
  });

  it("clamps the column when the target row is shorter, in both directions", () => {
    render(<Grid rows={[5, 2, 4]} />);
    screen.getByText("0.4").focus();
    press("ArrowDown");
    expect(document.activeElement).toBe(screen.getByText("1.1"));
    press("ArrowDown");
    expect(document.activeElement).toBe(screen.getByText("2.1"));
  });

  it("clamps the active item when rows shrink", () => {
    const { rerender } = render(<Grid rows={[5]} />);
    screen.getByText("0.4").focus();
    rerender(<Grid rows={[2]} />);
    expect(screen.getByText("0.1").tabIndex).toBe(0);
  });
});
