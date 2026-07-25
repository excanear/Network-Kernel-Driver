"use client";

import type { InterfaceStats } from "@/lib/types";
import { statusColor, theme } from "@/styles/theme";

function humanSpeed(bps: number | null): string {
  if (!bps) return "—";
  return `${(bps / 1_000_000).toFixed(0)} Mbps`;
}

interface Props {
  interfaces: InterfaceStats[];
  selectedIndex: number | null;
  onSelect: (index: number) => void;
}

export function InterfaceList({ interfaces, selectedIndex, onSelect }: Props) {
  return (
    <div
      style={{
        background: theme.surface,
        border: `1px solid ${theme.border}`,
        borderRadius: 8,
        overflow: "hidden",
      }}
    >
      <table>
        <thead>
          <tr style={{ borderBottom: `1px solid ${theme.gridline}` }}>
            {["", "Interface", "Status", "Speed", "MAC", "MTU", "IP"].map((h) => (
              <th
                key={h}
                style={{
                  textAlign: "left",
                  padding: "10px 12px",
                  color: theme.textMuted,
                  fontSize: 12,
                  fontWeight: 500,
                  textTransform: "uppercase",
                  letterSpacing: 0.4,
                }}
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {interfaces.map((iface) => {
            const selected = iface.index === selectedIndex;
            return (
              <tr
                key={iface.index}
                onClick={() => onSelect(iface.index)}
                style={{
                  cursor: "pointer",
                  background: selected ? "rgba(57,135,229,0.12)" : "transparent",
                  borderBottom: `1px solid ${theme.gridline}`,
                }}
              >
                <td style={{ padding: "10px 12px" }}>
                  <span
                    aria-hidden
                    style={{
                      display: "inline-block",
                      width: 8,
                      height: 8,
                      borderRadius: "50%",
                      background: statusColor(iface.oper_status),
                    }}
                  />
                </td>
                <td style={{ padding: "10px 12px" }}>
                  <div style={{ color: theme.textPrimary }}>{iface.name}</div>
                  <div style={{ color: theme.textMuted, fontSize: 12 }}>
                    {iface.description || iface.if_type}
                  </div>
                </td>
                <td style={{ padding: "10px 12px", color: statusColor(iface.oper_status) }}>
                  {iface.oper_status}
                </td>
                <td className="tabular-nums" style={{ padding: "10px 12px", color: theme.textSecondary }}>
                  {humanSpeed(iface.link_speed_bps)}
                </td>
                <td className="tabular-nums" style={{ padding: "10px 12px", color: theme.textSecondary }}>
                  {iface.mac_address || "—"}
                </td>
                <td className="tabular-nums" style={{ padding: "10px 12px", color: theme.textSecondary }}>
                  {iface.mtu}
                </td>
                <td style={{ padding: "10px 12px", color: theme.textSecondary, fontSize: 12 }}>
                  {[...iface.ipv4_addresses, ...iface.ipv6_addresses][0] ?? "—"}
                </td>
              </tr>
            );
          })}
          {interfaces.length === 0 && (
            <tr>
              <td colSpan={7} style={{ padding: 24, textAlign: "center", color: theme.textMuted }}>
                Waiting for data from network-observatoryd…
              </td>
            </tr>
          )}
        </tbody>
      </table>
      <div style={{ padding: "8px 12px", fontSize: 11, color: theme.textMuted }}>
        Click a row to select it for the throughput chart below.
      </div>
    </div>
  );
}
