<div align="center">

# 🛰️ Network Observatory

**Uma plataforma de observabilidade de interface de rede de nível enterprise.**
Métricas em tempo real, health score, alertas, mapeamento de topologia e relatórios para toda interface de rede de uma máquina Windows ou Linux — de drivers em modo kernel até APIs REST/gRPC/WebSocket, passando por dashboards web, desktop e CLI.

[![CI](https://github.com/excanear/network-observatory/actions/workflows/ci.yml/badge.svg)](https://github.com/excanear/network-observatory/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![Next.js](https://img.shields.io/badge/Next.js-14-black?logo=next.js)](https://nextjs.org/)
[![.NET](https://img.shields.io/badge/.NET-8-512BD4?logo=dotnet)](https://dotnet.microsoft.com/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](#licença)

[Arquitetura](docs/architecture.md) · [Especificação da API](docs/api-spec.md) · [Roadmap](docs/roadmap.md) · [Design do Driver de Kernel](docs/phase2-kernel-driver-design.md)

</div>

---

## O que é isto

O Network Observatory observa toda interface de rede de um host — contadores reais, estado de link real, saúde real, alertas reais — e expõe isso pela interface que fizer sentido: um terminal, um navegador, um app desktop nativo, ou outro serviço falando gRPC. É estritamente uma plataforma de **observabilidade**: nada neste código intercepta, modifica ou atrasa um único pacote. Toda camada, desde os drivers de kernel, foi desenhada como um observador somente leitura.

Construído como um time de plataforma construiria para produção: um serviço Rust de verdade com uma camada de armazenamento plugável (SQLite ou PostgreSQL/TimescaleDB), drivers reais em modo kernel tanto no Windows (filtro NDIS 6.30) quanto no Linux (módulo de kernel com genetlink), um sistema de plugins isolado por processo, autenticação multiusuário com argon2, logs estruturados com rotação e trilha de auditoria, e um CI que de fato compila e testa a stack inteira a cada push.

## Matriz de funcionalidades

| Camada | O que é real |
|---|---|
| **Drivers de kernel** | Filtro NDIS 6.30 real no Windows (`driver/windows`) — compila, linka e assina (test-sign). Módulo de kernel genetlink no Linux (`driver/linux`) — compila contra headers reais do kernel, **carrega de verdade via `insmod`**, verificado servindo dados reais de interface via Netlink. Ambos caem de volta graciosamente para coleta em modo usuário quando não instalados. |
| **Coleta** | Coletores em modo usuário sem depender de driver, via IP Helper API (Windows) e procfs/sysfs (Linux) — contadores, velocidade de link, MTU, MAC, IPv4/IPv6 reais desde o momento em que o serviço sobe. |
| **API** | REST, WebSocket e gRPC (`tonic`) — as três compartilham um único loop de coleta, então todo cliente vê exatamente os mesmos números no mesmo instante. |
| **CLI** | `network status / interfaces / monitor / history / health / alerts / topology / export / plugins` — modelada na UX do `nvidia-smi` / `kubectl` / `docker`. |
| **Dashboard web** | Next.js + TypeScript, tema escuro, gráfico de throughput ao vivo (ECharts), grafo de topologia interativo (Cytoscape.js), tela de login real. |
| **Dashboard desktop** | App WPF nativo (.NET 8) consumindo a mesma API REST — sem lógica de negócio duplicada. |
| **Health e alertas** | Health score de 0-100 por interface (disponibilidade, estabilidade, perda, erros) e um motor de regras com estado (link down, perda de pacotes, mudança de IP/velocidade) com resolução automática. |
| **Topologia** | Gateway, DNS e adaptadores virtuais (Hyper-V, Docker, WSL, VMware, VirtualBox, VPN) detectados automaticamente e renderizados como um grafo interativo. |
| **Relatórios** | CSV, JSON, HTML, Markdown e PDF — gerados sob demanda via REST ou `network export`. |
| **Plugins** | Plugins como processos independentes de qualquer linguagem, falando JSON via stdout — sem ABI insegura de biblioteca dinâmica. Vem com um exemplo real (sonda de latência/jitter via TCP-connect). |
| **Autenticação** | Contas multiusuário, hash de senha com argon2, cookies de sessão HttpOnly, login/logout auditados, bootstrap de admin no primeiro start (padrão Grafana). |
| **Armazenamento** | SQLite por padrão; PostgreSQL + TimescaleDB via uma única variável de ambiente, mesmo trait, zero mudança de código em outro lugar. |
| **Operação** | Roda como Windows Service de verdade (integrado à SCM) ou daemon `systemd` no Linux, logs em arquivo com rotação, e trilha de auditoria persistida. |
| **Qualidade** | 27 testes automatizados (Rust + web) rodando em Windows e Linux, CI no GitHub Actions travando todo push. |

Veja [`docs/roadmap.md`](docs/roadmap.md) para o histórico completo de construção fase a fase e o que foi deliberadamente adiado (um módulo de IA, assinatura de driver de produção, rollups de histórico multi-intervalo).

## Arquitetura

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Web (Next.js)          Desktop (WPF)          CLI (network ...)         │
└─────────────┬───────────────────┬───────────────────┬───────────────────┘
              │                   │                   │
              ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              REST · WebSocket · gRPC   (crates/service, axum + tonic)   │
│   auth · health · alertas · topologia · relatórios · plugins · auditoria │
└─────────────┬─────────────────────────────────────────────────────────┬─┘
              │                                                         │
              ▼                                                         ▼
   store::HistoryStore (SQLite | Postgres/TimescaleDB)      alerts::AlertEngine
              ▲
              │
┌─────────────┴─────────────────────────────────────────────────────────┐
│         collector_core::InterfaceCollector  (um único trait comum)      │
│  ┌────────────────────────┐            ┌────────────────────────────┐  │
│  │ Windows                │            │ Linux                      │  │
│  │  · IP Helper API (user)│            │  · procfs/sysfs (user)     │  │
│  │  · driver NDIS 6.30    │            │  · módulo genetlink netobs │  │
│  └────────────────────────┘            └────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────┘
```

Detalhamento completo das seis camadas, racional de design e internals dos drivers de kernel: [`docs/architecture.md`](docs/architecture.md) · [`docs/phase2-kernel-driver-design.md`](docs/phase2-kernel-driver-design.md).

## Stack técnica

| Área | Stack |
|---|---|
| Serviço | Rust, Tokio, Axum, Tonic (gRPC), rusqlite, `postgres` |
| Kernel — Windows | C, WDK, NDIS 6.30 (Lightweight Filter) |
| Kernel — Linux | C, kbuild, Netlink genérico (`genl`) |
| Web | Next.js 14, React, TypeScript, ECharts, Cytoscape.js, Vitest |
| Desktop | .NET 8, WPF |
| CLI | Rust, Clap |
| Armazenamento | SQLite, PostgreSQL + TimescaleDB |
| CI | GitHub Actions (matriz Windows + Linux) |

## Quickstart

**Pré-requisitos:** [Rust](https://rustup.rs) (stable) e [Node.js](https://nodejs.org) 20+.

```powershell
# 1. Compilar o workspace Rust
cargo build --workspace

# 2. Rodar o serviço (REST/WS na :7878, gRPC na :50051)
cargo run -p service
# ou: .\scripts\dev-service.ps1

# 3. Em outro terminal, usar a CLI
cargo run -p cli -- status
cargo run -p cli -- interfaces
cargo run -p cli -- monitor
cargo run -p cli -- topology
cargo run -p cli -- export --format html

# 4. Rodar o dashboard web
cd web
npm install
npm run dev
# → http://localhost:3000 (o primeiro start loga uma senha de admin gerada)

# 5. Ou rodar o dashboard desktop nativo
cd desktop/NetworkObservatory.Desktop
dotnet run
```

Apontar o serviço para PostgreSQL/TimescaleDB em vez de SQLite:

```powershell
$env:NETOBS_DATABASE_URL = "postgres://user:pass@host:5432/network_observatory"
cargo run -p service
```

## Estrutura do repositório

```
crates/
  collector-core        # modelo InterfaceStats/Snapshot + trait InterfaceCollector
  collector-windows      collector-linux     # backends reais por plataforma, sem driver
  store                  # HistoryStore (SQLite + Postgres/TimescaleDB), auth, auditoria, alertas
  health                 alerts              topology            reports
  plugin-api             service             cli
driver/
  windows                # driver de filtro NDIS 6.30
  linux                  # módulo de kernel genetlink
plugins/
  ping-latency-collector # exemplo de plugin (sonda de latência via TCP-connect)
web/                      # dashboard Next.js
desktop/NetworkObservatory.Desktop/  # dashboard WPF
docs/                     # arquitetura, modelo de dados, spec de API, design do driver, roadmap
scripts/                  # helpers de dev/instalação (Windows Service, systemd, Postgres/kernel via WSL2)
```

## Documentação

- [Arquitetura (seis camadas)](docs/architecture.md)
- [Design da fatia vertical da Fase 1](docs/phase1-slice-design.md)
- [Design do driver de kernel (NDIS Windows + genetlink Linux)](docs/phase2-kernel-driver-design.md)
- [Modelo de dados](docs/data-model.md)
- [Especificação da API (REST / WebSocket / gRPC)](docs/api-spec.md)
- [Roadmap e histórico de construção](docs/roadmap.md)
- [Notas do app desktop](desktop/README.md)
- [Build/instalação do driver Windows](driver/windows/README.md) · [Build/carregamento do driver Linux](driver/linux/README.md)

## Segurança

- Estritamente observacional: nenhuma interceptação, modificação ou injeção de tráfego em nenhum ponto da stack — ver a nota "fora de escopo permanente" em [`docs/roadmap.md`](docs/roadmap.md).
- Privilégio mínimo: a coleta em modo usuário não exige elevação; só a *instalação* do driver (não a operação) exige admin/root.
- Buffers de IOCTL/Netlink em modo kernel são validados por tamanho antes do uso; nenhum ponteiro de modo usuário é dereferenciado diretamente.
- Senhas com hash argon2; sessões são cookies HttpOnly; enforcement na API REST é opt-in via `NETOBS_AUTH_REQUIRED`.
- Todo login, alerta e execução de plugin é gravado numa trilha de auditoria (`GET /api/v1/audit`).

## Licença

MIT — ver metadados do workspace em `Cargo.toml`.
