# Arquitetura — Network Observatory

Plataforma de observabilidade da interface de rede: coleta, histórico, saúde,
alertas e análise. **Fora de escopo, por design**: interceptação, modificação ou
exploração de tráfego.

## Visão em 6 camadas

```
┌──────────────────────────────────────────────────────────────────────┐
│ Camada 6 — Web Dashboard (Next.js/React/TS, D3/ECharts/Cytoscape.js)│
├──────────────────────────────────────────────────────────────────────┤
│ Camada 5 — Desktop Dashboard (WPF, .NET 8)                          │
├──────────────────────────────────────────────────────────────────────┤
│ Camada 4 — CLI (`network ...`, estilo nvidia-smi/kubectl/docker/git)│
├──────────────────────────────────────────────────────────────────────┤
│ Camada 3 — API: REST · gRPC · WebSocket                             │
├──────────────────────────────────────────────────────────────────────┤
│ Camada 2 — Network Service (Rust; Windows Service / Linux daemon)   │
│   comunicação com driver · cache · histórico · banco · alertas ·    │
│   IA · logs · plugins                                               │
├──────────────────────────────────────────────────────────────────────┤
│ Camada 1 — Network Driver                                            │
│   Fase 1: APIs de SO (IP Helper API / procfs+sysfs) — sem driver     │
│           customizado, dados reais, zero risco de toolchain          │
│   Fase 2: Driver de modo kernel real                                 │
│           Windows: C++/WDK/KMDF/NDIS + ETW                           │
│           Linux:   C, Linux Kernel Module + Netlink                  │
└──────────────────────────────────────────────────────────────────────┘
```

## Por que a Camada 1 tem dois estágios

Um driver KMDF/NDIS assinado e um módulo de kernel Linux carregável são
investimentos de alto risco: exigem toolchain completo (WDK, headers de kernel
compatíveis com a versão rodando), processo de assinatura/test-signing, e
falhas podem gerar BSOD/kernel panic — nada disso itera rápido.

Os dados que a Fase 1 precisa (contadores, estado de link, velocidade, MTU,
endereços) já são expostos por APIs de modo usuário sem privilégio elevado:
- **Windows**: IP Helper API (`iphlpapi.dll`) — `GetIfTable2`, `GetIfEntry2`,
  `GetAdaptersAddresses`.
- **Linux**: `/proc/net/dev` (contadores) + `/sys/class/net/<if>/*` (estado,
  velocidade, duplex, MTU, endereço).

Por isso a Camada 1 é abstraída atrás do trait `InterfaceCollector`
(`crates/collector-core`), com dois backends reais hoje (`collector-windows`,
`collector-linux`) e um terceiro backend (`KernelDriver`) a ser adicionado na
Fase 2 sem alterar nenhuma camada acima — o contrato de dados
(`InterfaceStats`/`Snapshot`) já é o mesmo que o driver kernel produzirá.

## Camada 2 — Network Service

✅ Suporte real a **Windows Service** (`crates/service/src/win_service.rs`,
flag `--service`, integração via crate `windows-service` com a Service
Control Manager) e **daemon systemd** no Linux
(`scripts/network-observatoryd.service` + `scripts/install-linux-daemon.sh`,
usuário dedicado sem privilégios, `ProtectSystem=strict`). Instalação via
`scripts/install-windows-service.ps1` (Windows, requer elevação) ou
`scripts/install-linux-daemon.sh` (Linux, requer sudo) — nenhum dos dois é
executado automaticamente, pois altera estado persistente do sistema
operacional.

Binário `network-observatoryd` (Rust, tokio). Único processo que fala com a
Camada 1 e é a fonte única de verdade para as camadas acima:

- Loop de poll periódico (coleta → ring buffer em memória → broadcast para
  WebSocket → downsample periódico para o histórico persistido).
- Cache em memória para leituras de baixa latência.
- Histórico via `store::HistoryStore` (SQLite hoje; Postgres/TimescaleDB no
  roadmap, mesma interface).
- Hooks para alertas, IA, logs estruturados e plugins (Fase 3, ver roadmap).

## Camada 3 — API

- **REST** (`/api/v1/...`) — snapshots atuais e consultas de histórico.
- **WebSocket** (`/api/v1/ws/interfaces`) — streaming de snapshots em tempo real.
- **gRPC** — implementado via `tonic` em `grpc://127.0.0.1:50051`, contrato em
  [`api-spec.md`](api-spec.md), compartilhando a mesma fonte de coleta que o
  REST/WS.

## Camada 4 — CLI

Binário `network`, modelado em `nvidia-smi`/`kubectl`/`docker`/`git`:
subcomandos verbais, saída tabular por padrão, `--json` para automação.
Ver [`api-spec.md`](api-spec.md) para os comandos implementados na Fase 1.

## Camadas 5 e 6 — Dashboards

- ✅ **Web**: Next.js/React/TypeScript, tema escuro, ECharts para séries
  temporais, Cytoscape.js para o grafo de topologia, login real (`AuthGate`).
- ✅ **Desktop** (`desktop/NetworkObservatory.Desktop`, WPF — ver
  `desktop/README.md` para por que não é WinUI 3): consome a mesma API REST
  do web/CLI, sem lógica de negócio duplicada.

## Segurança

- Camada 2 roda com privilégio mínimo necessário; as APIs de SO usadas
  (Fase 1) são de leitura e não privilegiadas, sem exigir elevação.
- ✅ Autenticação real (`crates/store/src/auth.rs`): senhas com argon2,
  sessões via cookie HttpOnly + SameSite=Lax, CORS restrito a uma origem
  explícita com credentials. Cookie `Secure` fica pendente para quando o
  serviço rodar atrás de TLS (hoje é HTTP local); enforcement de sessão nas
  rotas REST é opt-in (`NETOBS_AUTH_REQUIRED`) — ver Fase H no roadmap.
- ✅ Trilha de auditoria real (`crates/store/src/audit.rs`,
  `GET /api/v1/audit`) e logs com rotação diária (`tracing-appender`).
- O driver Windows real (Fase L) valida todo IOCTL por tamanho exato via
  `METHOD_BUFFERED` (buffer nunca é um ponteiro de user-mode dereferenciado
  diretamente) e é um observador puro — nunca inspeciona, atrasa ou reescreve
  pacotes (`docs/phase2-kernel-driver-design.md`). Assinatura de produção
  (EV cert + WHQL) permanece um processo externo, fora do que é automatizável
  aqui.
