import { describe, expect, it } from "vitest";
import type { AppError } from "@om/shared";

describe("remote", () => {
  it("resolves the shared workspace package", () => {
    const err: AppError = { code: "unreachable", message: "TV unreachable" };
    expect(err.message).toContain("TV");
  });
});
