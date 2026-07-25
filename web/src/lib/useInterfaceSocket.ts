"use client";

import { useEffect, useRef, useState } from "react";
import { WS_URL } from "./api";
import type { Snapshot } from "./types";

const RECONNECT_DELAY_MS = 2000;

export interface SocketState {
  snapshot: Snapshot | null;
  connected: boolean;
}

/** Subscribes to /api/v1/ws/interfaces, reconnecting with a fixed backoff on drop. */
export function useInterfaceSocket(): SocketState {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [connected, setConnected] = useState(false);
  const cancelledRef = useRef(false);

  useEffect(() => {
    cancelledRef.current = false;
    let socket: WebSocket | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    const connect = () => {
      if (cancelledRef.current) return;
      socket = new WebSocket(WS_URL);

      socket.onopen = () => setConnected(true);

      socket.onmessage = (event) => {
        try {
          const parsed: Snapshot = JSON.parse(event.data);
          setSnapshot(parsed);
        } catch {
          // Ignore malformed frames rather than tearing down the connection.
        }
      };

      socket.onclose = () => {
        setConnected(false);
        if (!cancelledRef.current) {
          reconnectTimer = setTimeout(connect, RECONNECT_DELAY_MS);
        }
      };

      socket.onerror = () => {
        socket?.close();
      };
    };

    connect();

    return () => {
      cancelledRef.current = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      socket?.close();
    };
  }, []);

  return { snapshot, connected };
}
