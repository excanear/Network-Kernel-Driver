// Dark-theme design tokens for the Network Observatory dashboard.
// Values are the validated defaults from the dataviz skill's reference
// palette (references/palette.md) — swap here to retarget a brand palette.

export const theme = {
  surface: "#1a1a19",
  page: "#0d0d0d",
  textPrimary: "#ffffff",
  textSecondary: "#c3c2b7",
  textMuted: "#898781",
  gridline: "#2c2c2a",
  baseline: "#383835",
  border: "rgba(255,255,255,0.10)",

  // Categorical slots 1 (rx) and 2 (tx) — fixed order, not cycled.
  seriesRx: "#3987e5",
  seriesTx: "#d95926",

  status: {
    good: "#0ca30c", // Up
    warning: "#fab219", // Testing / Dormant
    serious: "#ec835a", // LowerLayerDown
    critical: "#d03b3b", // Down / NotPresent
    muted: "#898781", // Unknown
  },
} as const;

export function statusColor(status: string): string {
  switch (status) {
    case "Up":
      return theme.status.good;
    case "Down":
    case "NotPresent":
      return theme.status.critical;
    case "LowerLayerDown":
      return theme.status.serious;
    case "Testing":
    case "Dormant":
      return theme.status.warning;
    default:
      return theme.status.muted;
  }
}
