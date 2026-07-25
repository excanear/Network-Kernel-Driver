"use client";

import Link from "next/link";
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
  return (
    <nav style={{ display: "flex", gap: 4, borderBottom: `1px solid ${theme.gridline}`, paddingBottom: 8 }}>
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
    </nav>
  );
}
