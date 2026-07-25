"use client";

import { useEffect, useState } from "react";
import { TopologyGraph } from "@/components/TopologyGraph";
import { Nav } from "@/components/Nav";
import { getTopology } from "@/lib/api";
import type { TopologyGraph as TopologyGraphData } from "@/lib/types";
import { theme } from "@/styles/theme";

export default function TopologyPage() {
  const [graph, setGraph] = useState<TopologyGraphData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getTopology()
      .then(setGraph)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <main style={{ minHeight: "100vh", padding: "24px 32px", maxWidth: 1200, margin: "0 auto" }}>
      <Nav active="topology" />
      <h1 style={{ fontSize: 22, fontWeight: 600, margin: "16px 0 4px", color: theme.textPrimary }}>
        Topology
      </h1>
      <p style={{ margin: "0 0 24px", color: theme.textMuted, fontSize: 13 }}>
        Auto-detected from active interfaces, default gateways, and configured DNS servers.
      </p>

      {error && <p style={{ color: theme.status.critical }}>{error}</p>}
      {!graph && !error && <p style={{ color: theme.textMuted }}>Loading…</p>}
      {graph && <TopologyGraph graph={graph} />}
    </main>
  );
}
