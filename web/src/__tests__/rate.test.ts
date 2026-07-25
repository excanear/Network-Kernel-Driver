import { describe, expect, it } from "vitest";
import { toRatePoints } from "@/lib/rate";
import type { InterfaceStats } from "@/lib/types";

function sample(overrides: Partial<InterfaceStats>): InterfaceStats {
  return {
    index: 1,
    name: "eth0",
    description: "",
    mac_address: "",
    mtu: 1500,
    oper_status: "Up",
    link_speed_bps: 1_000_000_000,
    duplex: "Full",
    if_type: "Ethernet",
    ipv4_addresses: [],
    ipv6_addresses: [],
    rx_bytes: 0,
    tx_bytes: 0,
    rx_packets: 0,
    tx_packets: 0,
    rx_errors: 0,
    tx_errors: 0,
    rx_drops: 0,
    tx_drops: 0,
    rx_broadcast_packets: null,
    rx_multicast_packets: null,
    timestamp: new Date().toISOString(),
    collector_source: "WindowsIpHelper",
    ...overrides,
  };
}

describe("toRatePoints", () => {
  it("computes bytes/sec from consecutive cumulative samples", () => {
    const t0 = "2026-01-01T00:00:00.000Z";
    const t1 = "2026-01-01T00:00:01.000Z"; // 1s later

    const history = [
      sample({ timestamp: t0, rx_bytes: 1000, tx_bytes: 500 }),
      sample({ timestamp: t1, rx_bytes: 3000, tx_bytes: 1500 }),
    ];

    const points = toRatePoints(history);
    expect(points).toHaveLength(1);
    expect(points[0].rxBps).toBeCloseTo(2000, 5);
    expect(points[0].txBps).toBeCloseTo(1000, 5);
  });

  it("clamps negative deltas (counter reset) to zero instead of going negative", () => {
    const t0 = "2026-01-01T00:00:00.000Z";
    const t1 = "2026-01-01T00:00:01.000Z";

    const history = [
      sample({ timestamp: t0, rx_bytes: 5000 }),
      sample({ timestamp: t1, rx_bytes: 100 }), // adapter reset
    ];

    const points = toRatePoints(history);
    expect(points[0].rxBps).toBe(0);
  });

  it("returns an empty array for fewer than two samples", () => {
    expect(toRatePoints([])).toEqual([]);
    expect(toRatePoints([sample({})])).toEqual([]);
  });

  it("skips a pair with a non-positive time delta", () => {
    const t0 = "2026-01-01T00:00:01.000Z";
    const t1 = "2026-01-01T00:00:00.000Z"; // out of order

    const history = [sample({ timestamp: t0 }), sample({ timestamp: t1 })];
    expect(toRatePoints(history)).toEqual([]);
  });
});
