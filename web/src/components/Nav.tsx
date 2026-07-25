"use client";

import Link from "next/link";
import { useAuth } from "@/components/AuthGate";
import { theme } from "@/styles/theme";

const LINKS = [
  { href: "/", key: "overview", label: "Overview" },
  { href: "/topology", key: "topology", label: "Topology" },
  { href: "/alerts", key: "alerts", label: "Alerts" },
] as const;

interface Props {
  active: (typeof LINKS)[number]["key"];
}

export function Nav({ active }: Props) {
  const { user, logout } = useAuth();

  return (
    <nav
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 4,
        borderBottom: `1px solid ${theme.gridline}`,
        paddingBottom: 8,
      }}
    >
      <div style={{ display: "flex", gap: 4 }}>
        {LINKS.map((link) => (
          <Link
            key={link.key}
            href={link.href}
            style={{
              padding: "6px 12px",
              borderRadius: 6,
              fontSize: 13,
              textDecoration: "none",
              color: active === link.key ? theme.textPrimary : theme.textMuted,
              background: active === link.key ? "rgba(57,135,229,0.12)" : "transparent",
            }}
          >
            {link.label}
          </Link>
        ))}
      </div>
      {user && (
        <div style={{ display: "flex", alignItems: "center", gap: 10, fontSize: 12, color: theme.textMuted }}>
          <span>{user.username}</span>
          <button
            onClick={() => logout()}
            style={{
              background: "transparent",
              border: `1px solid ${theme.border}`,
              borderRadius: 6,
              color: theme.textMuted,
              fontSize: 12,
              padding: "4px 10px",
              cursor: "pointer",
            }}
          >
            Sign out
          </button>
        </div>
      )}
    </nav>
  );
}
