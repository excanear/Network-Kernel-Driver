<div align="center">

# 🛰️ Network Observatory

**An enterprise-grade network interface observability platform.**
Real-time metrics, health scoring, alerting, topology mapping, and reporting for every network adapter on Windows and Linux — from kernel-mode drivers up through REST/gRPC/WebSocket APIs to web, desktop, and CLI dashboards.

[![CI](https://github.com/excanear/network-observatory/actions/workflows/ci.yml/badge.svg)](https://github.com/excanear/network-observatory/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![Next.js](https://img.shields.io/badge/Next.js-14-black?logo=next.js)](https://nextjs.org/)
[![.NET](https://img.shields.io/badge/.NET-8-512BD4?logo=dotnet)](https://dotnet.microsoft.com/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](#license)

[Architecture](docs/architecture.md) · [API Spec](docs/api-spec.md) · [Roadmap](docs/roadmap.md) · [Kernel Driver Design](docs/phase2-kernel-driver-design.md)

</div>

---

## What this is

Network Observatory watches every network interface on a host — real counters, real link state, real health, real alerts — and surfaces it through whichever interface fits the job: a terminal, a browser, a native desktop app, or another service talking gRPC. It is strictly an **observability** platform: nothing in this codebase intercepts, modifies, or delays a single packet. Every layer, from the kernel drivers up, is designed as a read-only observer.

It's built the way a platform team would build it for production: a real Rust service backed by a pluggable storage layer (SQLite or PostgreSQL/TimescaleDB), real kernel-mode drivers on both Windows (NDIS 6.30 lightweight filter) and Linux (a genetlink kernel module), a process-isolated plugin system, argon2-backed multi-user auth, structured rotating logs with an audit trail, and CI that actually builds and tests the whole stack on every push.

## Feature matrix

| Layer | What's real |
|---|---|
| **Kernel drivers** | Windows NDIS 6.30 lightweight filter (`driver/windows`) — compiles, links, test-signs. Linux genetlink kernel module (`driver/linux`) — compiles against real kernel headers, **loads via `insmod`**, verified serving live interface data over Netlink. Both fall back gracefully to user-mode collection when not installed. |
| **Collection** | Zero-driver-required user-mode collectors via the Windows IP Helper API and Linux procfs/sysfs — real counters, link speed, MTU, MAC, IPv4/IPv6, from the moment the service starts. |
| **API** | REST, WebSocket, and gRPC (`tonic`) — all three share one poll loop, so every client sees the exact same numbers at the exact same instant. |
| **CLI** | `network status / interfaces / monitor / history / health / alerts / topology / export / plugins` — modeled on `nvidia-smi` / `kubectl` / `docker` UX. |
| **Web dashboard** | Next.js + TypeScript, dark theme, live throughput charts (ECharts), interactive topology graph (Cytoscape.js), real login gate. |
| **Desktop dashboard** | Native WPF (.NET 8) app consuming the same REST API — no duplicated business logic. |
| **Health & alerting** | Per-interface 0–100 health score (availability, stability, loss, errors) and a stateful rule engine (link down, packet loss, IP/speed change) with automatic resolution. |
| **Topology** | Auto-detected gateway, DNS, and virtual adapters (Hyper-V, Docker, WSL, VMware, VirtualBox, VPN) rendered as an interactive graph. |
| **Reports** | CSV, JSON, HTML, Markdown, and PDF — generated on demand via REST or `network export`. |
| **Plugins** | Language-agnostic process plugins speaking JSON over stdout — no unsafe dynamic-library ABI. Ships with a real example (TCP-connect latency/jitter probe). |
| **Auth** | Multi-user accounts, argon2 password hashing, HttpOnly session cookies, audited login/logout, Grafana-style admin bootstrap on first run. |
| **Storage** | SQLite by default; PostgreSQL + TimescaleDB via one environment variable, same trait, zero code changes elsewhere. |
| **Ops** | Runs as a real Windows Service (SCM-integrated) or Linux `systemd` daemon, rotating file logs, and a persisted audit trail. |
| **Quality** | 27 automated tests (Rust + web) across both Windows and Linux, GitHub Actions CI gating every push. |

See [`docs/roadmap.md`](docs/roadmap.md) for the full phase-by-phase build log and what's intentionally deferred (an AI insights module, production driver signing, multi-interval history rollups).

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Web (Next.js)          Desktop (WPF)          CLI (network ...)         │
└─────────────┬───────────────────┬───────────────────┬───────────────────┘
              │                   │                   │
              ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              REST · WebSocket · gRPC   (crates/service, axum + tonic)   │
│   auth · health · alerts · topology · reports · plugins · audit log     │
└─────────────┬─────────────────────────────────────────────────────────┬─┘
              │                                                         │
              ▼                                                         ▼
   store::HistoryStore (SQLite | Postgres/TimescaleDB)      alerts::AlertEngine
              ▲
              │
┌─────────────┴─────────────────────────────────────────────────────────┐
│           collector_core::InterfaceCollector  (one shared trait)        │
│  ┌────────────────────────┐            ┌────────────────────────────┐  │
│  │ Windows                │            │ Linux                      │  │
│  │  · IP Helper API (user)│            │  · procfs/sysfs (user)     │  │
│  │  · NDIS 6.30 LWF driver│            │  · netobs genetlink module │  │
│  └────────────────────────┘            └────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────┘
```

Full six-layer breakdown, design rationale, and the kernel driver internals: [`docs/architecture.md`](docs/architecture.md) · [`docs/phase2-kernel-driver-design.md`](docs/phase2-kernel-driver-design.md).

## Tech stack

| Area | Stack |
|---|---|
| Service | Rust, Tokio, Axum, Tonic (gRPC), rusqlite, `postgres` |
| Kernel — Windows | C, WDK, NDIS 6.30 (Lightweight Filter) |
| Kernel — Linux | C, kbuild, generic Netlink (`genl`) |
| Web | Next.js 14, React, TypeScript, ECharts, Cytoscape.js, Vitest |
| Desktop | .NET 8, WPF |
| CLI | Rust, Clap |
| Storage | SQLite, PostgreSQL + TimescaleDB |
| CI | GitHub Actions (Windows + Linux matrix) |

## Quickstart

**Prerequisites:** [Rust](https://rustup.rs) (stable) and [Node.js](https://nodejs.org) 20+.

```powershell
# 1. Build the Rust workspace
cargo build --workspace

# 2. Run the service (REST/WS on :7878, gRPC on :50051)
cargo run -p service
# or: .\scripts\dev-service.ps1

# 3. In another terminal, drive it from the CLI
cargo run -p cli -- status
cargo run -p cli -- interfaces
cargo run -p cli -- monitor
cargo run -p cli -- topology
cargo run -p cli -- export --format html

# 4. Run the web dashboard
cd web
npm install
npm run dev
# → http://localhost:3000 (first run logs a bootstrapped admin password)

# 5. Or run the native desktop dashboard
cd desktop/NetworkObservatory.Desktop
dotnet run
```

Point the service at PostgreSQL/TimescaleDB instead of SQLite:

```powershell
$env:NETOBS_DATABASE_URL = "postgres://user:pass@host:5432/network_observatory"
cargo run -p service
```

## Repository structure

```
crates/
  collector-core        # InterfaceStats/Snapshot model + InterfaceCollector trait
  collector-windows      collector-linux     # real, driver-free platform backends
  store                  # SQLite + Postgres/TimescaleDB HistoryStore, auth, audit, alerts
  health                 alerts              topology            reports
  plugin-api             service             cli
driver/
  windows                # NDIS 6.30 lightweight filter driver
  linux                  # genetlink kernel module
plugins/
  ping-latency-collector # example process plugin (TCP-connect latency probe)
web/                      # Next.js dashboard
desktop/NetworkObservatory.Desktop/  # WPF dashboard
docs/                     # architecture, data model, API spec, kernel driver design, roadmap
scripts/                  # dev/install helpers (Windows Service, systemd, WSL2 Postgres/kernel)
```

## Documentation

- [Architecture (six layers)](docs/architecture.md)
- [Phase 1 vertical-slice design](docs/phase1-slice-design.md)
- [Kernel driver design (Windows NDIS + Linux genetlink)](docs/phase2-kernel-driver-design.md)
- [Data model](docs/data-model.md)
- [API specification (REST / WebSocket / gRPC)](docs/api-spec.md)
- [Roadmap & build log](docs/roadmap.md)
- [Desktop app notes](desktop/README.md)
- [Windows driver build/install](driver/windows/README.md) · [Linux driver build/load](driver/linux/README.md)

## Security

- Strictly observational: no traffic interception, modification, or injection anywhere in the stack — see the "out of permanent scope" note in [`docs/roadmap.md`](docs/roadmap.md).
- Least privilege: user-mode collection needs no elevation; only driver *installation* (not operation) requires admin/root.
- Kernel-mode IOCTL/Netlink buffers are size-validated before use; no direct dereference of user-mode pointers.
- Passwords hashed with argon2; sessions are HttpOnly cookies; REST enforcement is opt-in via `NETOBS_AUTH_REQUIRED`.
- Every login, alert, and plugin execution is written to an audit trail (`GET /api/v1/audit`).

## License

MIT — see `Cargo.toml` workspace metadata.
