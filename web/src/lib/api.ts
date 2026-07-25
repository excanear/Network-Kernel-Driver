import type { Snapshot, StatusResponse } from "./types";

const BASE_URL =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:7878/api/v1";

export const WS_URL =
  process.env.NEXT_PUBLIC_WS_URL ?? "ws://localhost:7878/api/v1/ws/interfaces";

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`GET ${path} failed: ${res.status} ${res.statusText}`);
  }
  return res.json() as Promise<T>;
}

export function getStatus(): Promise<StatusResponse> {
  return getJson<StatusResponse>("/status");
}

export function getInterfaces(): Promise<Snapshot> {
  return getJson<Snapshot>("/interfaces");
}
