"use client";

import cytoscape, { type ElementDefinition } from "cytoscape";
import { useEffect, useRef } from "react";
import type { NodeType, TopologyGraph as TopologyGraphData } from "@/lib/types";
import { theme } from "@/styles/theme";

const NODE_COLOR: Record<NodeType, string> = {
  Host: theme.textPrimary,
  Nic: theme.seriesRx,
  Gateway: theme.status.warning,
  Dns: theme.seriesTx,
  Vpn: theme.status.serious,
  HyperV: "#9085e9",
  Docker: "#1baf7a",
  Wsl: "#1baf7a",
  VMware: "#eda100",
  VirtualBox: "#eda100",
  VirtualSwitch: theme.textMuted,
};

interface Props {
  graph: TopologyGraphData;
}

export function TopologyGraph({ graph }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const cyRef = useRef<cytoscape.Core | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const elements: ElementDefinition[] = [
      ...graph.nodes.map((n) => ({
        data: { id: n.id, label: n.label, nodeType: n.node_type },
      })),
      ...graph.edges.map((e, i) => ({
        data: { id: `e${i}`, source: e.from, target: e.to },
      })),
    ];

    const cy = cytoscape({
      container: containerRef.current,
      elements,
      style: [
        {
          selector: "node",
          style: {
            "background-color": (ele) => NODE_COLOR[ele.data("nodeType") as NodeType] ?? theme.textMuted,
            label: "data(label)",
            color: theme.textSecondary,
            "font-size": 11,
            "text-valign": "bottom",
            "text-margin-y": 6,
            width: 28,
            height: 28,
            "border-width": 2,
            "border-color": theme.border,
          },
        },
        {
          selector: "edge",
          style: {
            width: 1.5,
            "line-color": theme.gridline,
            "target-arrow-color": theme.gridline,
            "target-arrow-shape": "triangle",
            "curve-style": "bezier",
          },
        },
      ],
      layout: { name: "breadthfirst", directed: true, spacingFactor: 1.3 },
      minZoom: 0.3,
      maxZoom: 2,
    });

    cyRef.current = cy;
    return () => cy.destroy();
  }, [graph]);

  return (
    <div
      style={{
        background: theme.surface,
        border: `1px solid ${theme.border}`,
        borderRadius: 8,
        padding: 8,
      }}
    >
      <div ref={containerRef} style={{ width: "100%", height: 480 }} />
    </div>
  );
}
