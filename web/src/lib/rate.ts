import type { InterfaceStats } from "./types";

export interface RatePoint {
  time: number;
  rxBps: number;
  txBps: number;
}

/** Converts a chronologically-ascending list of cumulative-counter samples
 * into per-tick rate points (bytes/sec), matching the semantics the REST
 * history endpoint and the WebSocket stream both use. */
export function toRatePoints(history: InterfaceStats[]): RatePoint[] {
  const points: RatePoint[] = [];
  for (let i = 1; i < history.length; i += 1) {
    const prev = history[i - 1];
    const cur = history[i];
    const dtSeconds = (new Date(cur.timestamp).getTime() - new Date(prev.timestamp).getTime()) / 1000;
    if (dtSeconds <= 0) continue;
    const rxDelta = cur.rx_bytes - prev.rx_bytes;
    const txDelta = cur.tx_bytes - prev.tx_bytes;
    points.push({
      time: new Date(cur.timestamp).getTime(),
      rxBps: Math.max(0, rxDelta) / dtSeconds,
      txBps: Math.max(0, txDelta) / dtSeconds,
    });
  }
  return points;
}
