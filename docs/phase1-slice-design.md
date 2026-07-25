# Fase 1 — Design da Fatia Vertical

Objetivo: prova real, ponta a ponta, da arquitetura completa — sem mocks —
usando apenas APIs de SO já disponíveis (sem driver de modo kernel
customizado, adiado para a [Fase 2](phase2-kernel-driver-design.md)).

## Caminho de dados

```
SO (IP Helper API / procfs+sysfs)
   → collector-windows | collector-linux   (implementa InterfaceCollector)
      → poller (service, tokio, intervalo padrão 1s)
         ├─→ ring buffer em memória (store::memory)  → WebSocket broadcast
         └─→ downsample a cada ~10s                  → SQLite (store::sqlite)
              ↓
        REST (/api/v1/*) e WS (/api/v1/ws/interfaces)
              ↓                    ↓
             CLI                  Web (Next.js)
      (network status|interfaces|monitor|history)   (Live Overview)
```

Uma única coleta por tick alimenta memória, WS e (periodicamente) SQLite —
não há chamadas redundantes ao coletor por consumidor.

## Crates

- **`collector-core`**: `InterfaceStats`, `Snapshot`, trait `InterfaceCollector`,
  `CollectorError` — sem dependência de plataforma.
- **`collector-windows`** (`cfg(windows)`): backend via `windows` crate,
  chamando IP Helper API.
- **`collector-linux`** (`cfg(unix)`): parsing de `/proc/net/dev` +
  `/sys/class/net/*`, com `if-addrs` para IPs.
- **`store`**: `HistoryStore` trait + implementação em memória (ring buffer)
  e SQLite (`rusqlite`).
- **`service`** (bin `network-observatoryd`): `axum` + `tokio`, hospeda o
  poller, REST, WebSocket.
- **`cli`** (bin `network`): `clap`, cliente HTTP/WS do serviço.

## Web (`web/`)

Next.js (App Router) + TypeScript. Página única "Live Overview":
lista de interfaces (`GET /interfaces`) + gráfico de throughput em tempo real
(ECharts) via WebSocket, com seletor de interface. Tema escuro consistente.

## O que NÃO está nesta fase

Ver [`roadmap.md`](roadmap.md) para a lista completa (topologia, alertas, IA,
Postgres/TimescaleDB, plugins, relatórios, desktop, gRPC real, driver kernel
real).

## Verificação

1. `cargo build --workspace`
2. `cargo run -p service` — log deve mostrar backend `WindowsIpHelper`/`LinuxProcSys` e nº de interfaces > 0
3. `curl http://localhost:7878/api/v1/status` / `/interfaces` — comparar com `ipconfig /all` (Windows) ou `ip addr` (Linux)
4. `cargo run -p cli -- status|interfaces|monitor|history --interface <n>`
5. `cd web && npm install && npm run dev` → `http://localhost:3000`
6. Checklist end-to-end: SO → collector → poller → REST bate → WS bate → CLI bate → web bate, tudo mudando junto em tempo real com tráfego real.
