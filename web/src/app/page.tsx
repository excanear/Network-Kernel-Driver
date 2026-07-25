"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { InterfaceList } from "@/components/InterfaceList";
import { ThroughputChart } from "@/components/ThroughputChart";
import { getInterfaces } from "@/lib/api";
import type { InterfaceStats } from "@/lib/types";
import { useInterfaceSocket } from "@/lib/useInterfaceSocket";
import { theme } from "@/styles/theme";

const HISTORY_WINDOW = 300;

export default function LiveOverviewPage() {
  const { snapshot, connected } = useInterfaceSocket();
  const [initialInterfaces, setInitialInterfaces] = useState<InterfaceStats[]>([]);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const historyRef = useRef<Map<number, InterfaceStats[]>>(new Map());
  const [historyVersion, setHistoryVersion] = useState(0);

  useEffect(() => {
    getInterfaces()
      .then((snap) => setInitialInterfaces(snap.interfaces))
      .catch(() => {
        // The websocket connection will populate the list once the service is reachable.
      });
  }, []);

  useEffect(() => {
    if (!snapshot) return;
    for (const iface of snapshot.interfaces) {
      const existing = historyRef.current.get(iface.index) ?? [];
      const next = [...existing, iface].slice(-HISTORY_WINDOW);
      historyRef.current.set(iface.index, next);
    }
    setHistoryVersion((v) => v + 1);
  }, [snapshot]);

  const interfaces = snapshot?.interfaces ?? initialInterfaces;

  useEffect(() => {
    if (selectedIndex === null && interfaces.length > 0) {
      setSelectedIndex(interfaces[0].index);
    }
  }, [interfaces, selectedIndex]);

  const selectedHistory = useMemo(() => {
    if (selectedIndex === null) return [];
    return historyRef.current.get(selectedIndex) ?? [];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedIndex, historyVersion]);

  const selectedInterface = interfaces.find((i) => i.index === selectedIndex);

  return (
    <main style={{ minHeight: "100vh", padding: "24px 32px", maxWidth: 1200, margin: "0 auto" }}>
      <header style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between", marginBottom: 24 }}>
        <div>
          <h1 style={{ fontSize: 22, fontWeight: 600, margin: 0, color: theme.textPrimary }}>
            Network Observatory
          </h1>
          <p style={{ margin: "4px 0 0", color: theme.textMuted, fontSize: 13 }}>Live Overview</p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8, color: theme.textSecondary, fontSize: 13 }}>
          <span
            aria-hidden
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: connected ? theme.status.good : theme.status.critical,
              display: "inline-block",
            }}
          />
          {connected ? "Live" : "Reconnecting…"}
        </div>
      </header>

      <section style={{ marginBottom: 24 }}>
        <InterfaceList interfaces={interfaces} selectedIndex={selectedIndex} onSelect={setSelectedIndex} />
      </section>

      <section>
        <ThroughputChart history={selectedHistory} interfaceName={selectedInterface?.name ?? ""} />
      </section>
    </main>
  );
}
