# Network Observatory

Uma plataforma de observabilidade de interface de rede: coleta, histórico, health,
alertas e análise — não um interceptador/modificador de tráfego.

> Status: **Fase 1** — fatia vertical funcional (coleta real via APIs de SO → serviço
> Rust → REST/WebSocket → CLI → dashboard web). O driver de modo kernel real
> (Windows KMDF/NDIS, módulo de kernel Linux) é a **Fase 2**, já projetada em
> [`docs/phase2-kernel-driver-design.md`](docs/phase2-kernel-driver-design.md).

## Arquitetura

Veja [`docs/architecture.md`](docs/architecture.md) para a visão completa de 6 camadas
e [`docs/roadmap.md`](docs/roadmap.md) para o que vem depois da Fase 1.

```
collector-core (trait + modelo)
   ├── collector-windows  (IP Helper API — GetIfTable2/GetIfEntry2/GetAdaptersAddresses)
   └── collector-linux    (/proc/net/dev, /sys/class/net/*)
        │
        ▼
      store (ring buffer em memória + histórico SQLite)
        │
        ▼
     service (network-observatoryd — axum: REST + WebSocket)
        │           │
        ▼           ▼
       cli        web (Next.js, dashboard ao vivo)
   (network ...)
```

## Quickstart

Pré-requisitos: [Rust](https://rustup.rs) (stable) e [Node.js](https://nodejs.org) 20+.

```powershell
# 1. Compilar o workspace Rust
cargo build --workspace

# 2. Rodar o serviço (porta padrão 7878)
cargo run -p service
# ou: .\scripts\dev-service.ps1

# 3. Em outro terminal, usar a CLI
cargo run -p cli -- status
cargo run -p cli -- interfaces
cargo run -p cli -- monitor

# 4. Rodar o dashboard web
cd web
npm install
npm run dev
# abrir http://localhost:3000
```

## Estrutura do repositório

- `crates/collector-core` — modelo de dados (`InterfaceStats`, `Snapshot`) e trait `InterfaceCollector`.
- `crates/collector-windows` / `crates/collector-linux` — implementações reais por plataforma, sem driver customizado.
- `crates/store` — histórico em memória (ring buffer) e SQLite.
- `crates/service` — binário `network-observatoryd`: loop de coleta, REST, WebSocket.
- `crates/cli` — binário `network`, CLI estilo `nvidia-smi`/`kubectl`.
- `driver/windows`, `driver/linux` — placeholders para o driver de modo kernel real (Fase 2).
- `web/` — dashboard Next.js/React/TypeScript.
- `docs/` — arquitetura completa, modelo de dados, especificação de API e roadmap.

## Documentação

- [Arquitetura completa (6 camadas)](docs/architecture.md)
- [Design da fatia da Fase 1](docs/phase1-slice-design.md)
- [Design do driver kernel (Fase 2)](docs/phase2-kernel-driver-design.md)
- [Modelo de dados](docs/data-model.md)
- [Especificação de API (REST/WS/gRPC)](docs/api-spec.md)
- [Roadmap (Fases 3+)](docs/roadmap.md)
