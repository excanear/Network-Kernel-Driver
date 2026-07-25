"use client";

import * as echarts from "echarts";
import { useEffect, useRef } from "react";
import type { InterfaceStats } from "@/lib/types";
import { toRatePoints } from "@/lib/rate";
import { theme } from "@/styles/theme";

const WINDOW_SIZE = 300; // ~5 min at 1s poll interval, matches the service's ring buffer

interface Props {
  history: InterfaceStats[]; // ascending by timestamp, cumulative counters
  interfaceName: string;
}

export function ThroughputChart({ history, interfaceName }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;
    const chart = echarts.init(containerRef.current, "dark", { renderer: "svg" });
    chartRef.current = chart;

    const resize = () => chart.resize();
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      chart.dispose();
      chartRef.current = null;
    };
  }, []);

  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;

    const points = toRatePoints(history).slice(-WINDOW_SIZE);

    chart.setOption({
      backgroundColor: "transparent",
      textStyle: { color: theme.textSecondary, fontFamily: "system-ui, sans-serif" },
      grid: { left: 56, right: 24, top: 40, bottom: 32 },
      legend: {
        data: ["RX", "TX"],
        top: 0,
        right: 0,
        textStyle: { color: theme.textSecondary },
        itemWidth: 14,
        itemHeight: 8,
      },
      tooltip: {
        trigger: "axis",
        backgroundColor: theme.surface,
        borderColor: theme.border,
        textStyle: { color: theme.textPrimary },
        axisPointer: { type: "cross", lineStyle: { color: theme.baseline } },
        valueFormatter: (value: number) => `${(value / 1_000_000).toFixed(2)} Mbps`,
      },
      xAxis: {
        type: "time",
        axisLine: { lineStyle: { color: theme.baseline } },
        axisLabel: { color: theme.textMuted },
        splitLine: { show: false },
      },
      yAxis: {
        type: "value",
        name: "Mbps",
        nameTextStyle: { color: theme.textMuted },
        axisLabel: {
          color: theme.textMuted,
          formatter: (value: number) => `${(value / 1_000_000).toFixed(0)}`,
        },
        splitLine: { lineStyle: { color: theme.gridline } },
      },
      series: [
        {
          name: "RX",
          type: "line",
          showSymbol: false,
          lineStyle: { width: 2, color: theme.seriesRx },
          itemStyle: { color: theme.seriesRx },
          areaStyle: { color: theme.seriesRx, opacity: 0.08 },
          data: points.map((p) => [p.time, p.rxBps]),
        },
        {
          name: "TX",
          type: "line",
          showSymbol: false,
          lineStyle: { width: 2, color: theme.seriesTx },
          itemStyle: { color: theme.seriesTx },
          areaStyle: { color: theme.seriesTx, opacity: 0.08 },
          data: points.map((p) => [p.time, p.txBps]),
        },
      ],
    });
  }, [history]);

  return (
    <div
      style={{
        background: theme.surface,
        border: `1px solid ${theme.border}`,
        borderRadius: 8,
        padding: 16,
      }}
    >
      <div style={{ color: theme.textPrimary, fontSize: 14, marginBottom: 4 }}>
        Throughput — {interfaceName || "select an interface"}
      </div>
      <div ref={containerRef} style={{ width: "100%", height: 320 }} />
    </div>
  );
}
