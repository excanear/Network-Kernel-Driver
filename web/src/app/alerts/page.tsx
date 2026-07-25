"use client";

import { useEffect, useState } from "react";
import { Nav } from "@/components/Nav";
import { getActiveAlerts, getHealth } from "@/lib/api";
import type { Alert, HealthScore } from "@/lib/types";
import { theme } from "@/styles/theme";

function severityColor(severity: Alert["severity"]): string {
  switch (severity) {
    case "Critical":
      return theme.status.critical;
    case "Warning":
      return theme.status.warning;
    default:
      return theme.status.good;
  }
}

function scoreColor(score: number): string {
  if (score >= 90) return theme.status.good;
  if (score >= 70) return theme.status.warning;
  return theme.status.critical;
}

export default function AlertsPage() {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [health, setHealth] = useState<HealthScore[]>([]);

  useEffect(() => {
    const load = () => {
      getActiveAlerts().then(setAlerts).catch(() => {});
      getHealth().then(setHealth).catch(() => {});
    };
    load();
    const id = setInterval(load, 5000);
    return () => clearInterval(id);
  }, []);

  return (
    <main style={{ minHeight: "100vh", padding: "24px 32px", maxWidth: 1200, margin: "0 auto" }}>
      <Nav active="alerts" />
      <h1 style={{ fontSize: 22, fontWeight: 600, margin: "16px 0 24px", color: theme.textPrimary }}>
        Health &amp; Alerts
      </h1>

      <section style={{ marginBottom: 32 }}>
        <h2 style={{ fontSize: 14, color: theme.textMuted, textTransform: "uppercase", letterSpacing: 0.4 }}>
          Active alerts ({alerts.length})
        </h2>
        <div style={{ background: theme.surface, border: `1px solid ${theme.border}`, borderRadius: 8 }}>
          {alerts.length === 0 && (
            <div style={{ padding: 24, color: theme.textMuted, textAlign: "center" }}>No active alerts.</div>
          )}
          {alerts.map((alert) => (
            <div
              key={alert.id}
              style={{
                display: "flex",
                gap: 12,
                alignItems: "baseline",
                padding: "10px 16px",
                borderBottom: `1px solid ${theme.gridline}`,
              }}
            >
              <span
                style={{
                  color: severityColor(alert.severity),
                  fontSize: 11,
                  fontWeight: 600,
                  textTransform: "uppercase",
                  width: 70,
                }}
              >
                {alert.severity}
              </span>
              <span style={{ color: theme.textPrimary, minWidth: 160 }}>{alert.if_name}</span>
              <span style={{ color: theme.textSecondary, fontSize: 13 }}>{alert.message}</span>
            </div>
          ))}
        </div>
      </section>

      <section>
        <h2 style={{ fontSize: 14, color: theme.textMuted, textTransform: "uppercase", letterSpacing: 0.4 }}>
          Health score
        </h2>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))", gap: 12 }}>
          {health.map((h) => (
            <div
              key={h.if_index}
              style={{
                background: theme.surface,
                border: `1px solid ${theme.border}`,
                borderRadius: 8,
                padding: 16,
              }}
            >
              <div style={{ fontSize: 28, fontWeight: 600, color: scoreColor(h.score) }} className="tabular-nums">
                {h.score.toFixed(0)}
              </div>
              <div style={{ color: theme.textMuted, fontSize: 12, marginBottom: 8 }}>Interface {h.if_index}</div>
              <div style={{ color: theme.textSecondary, fontSize: 11 }} className="tabular-nums">
                availability {h.availability_pct.toFixed(0)}% · loss {h.packet_loss_pct.toFixed(1)}%
              </div>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
