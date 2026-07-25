import { describe, expect, it } from "vitest";
import { statusColor, theme } from "@/styles/theme";

describe("statusColor", () => {
  it("maps Up to the good status color", () => {
    expect(statusColor("Up")).toBe(theme.status.good);
  });

  it("maps Down and NotPresent to the critical status color", () => {
    expect(statusColor("Down")).toBe(theme.status.critical);
    expect(statusColor("NotPresent")).toBe(theme.status.critical);
  });

  it("falls back to muted for unrecognized status strings", () => {
    expect(statusColor("SomethingUnexpected")).toBe(theme.status.muted);
  });
});
