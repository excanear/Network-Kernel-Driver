# Roadmap

## Fase 1 — Fatia vertical funcional (concluída)

Coleta real (APIs de SO) → serviço Rust → REST + WebSocket → CLI → dashboard
web ao vivo. Ver [`phase1-slice-design.md`](phase1-slice-design.md).

## Fase 2 — Driver de modo kernel real

- ✅ **Windows**: filtro NDIS 6.30 real (`driver/windows/`) — compila e linka
  com sucesso contra o WDK (`cl.exe`/`link.exe` diretos, sem a extensão VS do
  WDK que não instalou nesta máquina), test-assinado com certificado próprio.
  **Falta apenas a instalação** (`install-test-driver.ps1`), que exige
  PowerShell elevado + reboot para test-signing — não executável pelo agente
  de automação nesta sessão (mesma classe de limitação do sudo/WSL2 na Fase
  F/M), documentado em `driver/windows/README.md`.
- ❌ Linux: módulo de kernel + família Netlink — ver Fase M.
- Backend `collector-kernel` plugável atrás do mesmo trait `InterfaceCollector`
  (contrato IOCTL já definido em `driver/windows/inc/ioctl_contract.h`; o
  crate Rust `collector-kernel-windows` que fala com `\\.\NetObsFilter` ainda
  não foi escrito — próximo passo depois da instalação do driver).

Design completo: [`phase2-kernel-driver-design.md`](phase2-kernel-driver-design.md).

## Fase 3 — Observabilidade avançada

- **Histórico multi-intervalo**: rollups automáticos (1m, 5m, 15m, 30m, 1h,
  6h, 12h, 24h, 7d, 30d, 365d) sobre a base já lançada na Fase 1 (SQLite →
  Postgres/TimescaleDB via `HistoryStore`), com comparação entre períodos.
- ✅ **Topologia** (`crates/topology`, `GET /api/v1/topology`, `network
  topology`, página web `/topology`): detecção automática de NIC ativa,
  gateway (via IP Helper API / `/proc/net/route`), DNS configurado (via
  `GetAdaptersAddresses` / `/etc/resolv.conf`), classificação heurística de
  interfaces virtuais (Hyper-V, Docker, WSL, VMware, VirtualBox, VPN/TAP/TUN),
  grafo interativo (Cytoscape.js). Detecção de switch físico fica fora de
  escopo (não observável sem SNMP/LLDP a um equipamento gerenciável).
- ✅ **Health score** (`crates/health`, `GET /api/v1/health[/:index]`, `network
  statistics`): disponibilidade, estabilidade (mudanças de link), perda de
  pacotes e taxa de erros, agregados num score 0-100 por interface a partir do
  histórico em memória. Temperatura/latência/jitter ficam para quando houver
  driver real (Fase 2) ou sondagem ativa (ping/RTT — ainda não implementada).
- ✅ **Motor de alertas** (`crates/alerts`, `GET /api/v1/alerts[/recent]`,
  `network alerts`): packet loss > X, taxa de erro > X, link down, interface
  offline, mudança de IP, velocidade negociada reduzida — com resolução
  automática quando a condição cessa. Gateway offline, DNS lento e mudança de
  gateway/DNS ficam para a Fase D (dependem de dados de topologia). Canais de
  notificação externos (email/webhook/Slack) permanecem no roadmap.
- ✅ **Relatórios** (`crates/reports`, `GET /api/v1/reports?format=`, `network
  export --format`): CSV/JSON/HTML/Markdown/PDF reais, com resumo executivo,
  métricas, gráfico de health (SVG no HTML), eventos/alertas e conclusões.
  Timeline detalhada por evento fica para quando o histórico multi-intervalo
  (Fase F) estiver pronto — hoje o relatório reflete o snapshot atual + janela
  recente de alertas.
- ✅ **Sistema de plugins** (`crates/plugin-api`, `GET /api/v1/plugins[/:name/run]`,
  `network plugins` / `network plugin-run`): plugins são executáveis
  independentes que falam JSON via stdout (`manifest`/`run`) — sem ABI de
  biblioteca dinâmica para manter estável entre versões do compilador. Exemplo
  real incluído: `plugins/ping-latency-collector`, medindo latência/jitter/
  perda via tempo de conexão TCP (proxy sem privilégio para RTT ICMP, que
  exigiria raw sockets elevados). Novos coletores/alertas/relatórios podem ser
  adicionados como novos executáveis, sem alterar o core. Dashboards/gráficos
  plugáveis na Camada 6 ficam para quando houver um mecanismo de widgets
  dinâmicos no web (não implementado nesta fase).
- ✅ **Logging estruturado com rotação** (`tracing-appender`, rotação diária em
  `logs/network-observatoryd.log.<data>`, além de stdout) e **trilha de
  auditoria** real (`crates/store/src/audit.rs`, `GET /api/v1/audit`):
  login/logout, falhas de login, alertas disparados/resolvidos e execuções de
  plugin ficam registrados com ator e timestamp.
- **Banco de dados**: suporte PostgreSQL/TimescaleDB como alternativa ao
  SQLite para séries temporais em escala.
- ✅ **Windows Service / daemon systemd real** (`crates/service/src/win_service.rs`,
  flag `--service`, `scripts/install-windows-service.ps1` /
  `scripts/install-linux-daemon.sh` + unit file): `network-observatoryd` pode
  rodar sob a Service Control Manager do Windows ou como daemon systemd no
  Linux, não só como processo de console.
- ✅ **Testes automatizados**: unitários em `collector-linux` (parsing de
  `/proc/net/dev`), `health`, `alerts` (motor de regras), `store` (SQLite
  histórico/alertas/auth em memória), `topology`, `reports` (todos os 5
  formatos), e testes Vitest no web (`rate.ts`, `theme.ts`) — todos passando
  em Windows e Linux (validado via WSL2).
- ✅ **CI** (`.github/workflows/ci.yml`): build+test Rust em matriz
  Windows/Linux, build+test do web em Ubuntu, a cada push/PR para `master`.
  Sem métricas de cobertura formais (`cargo-tarpaulin`/`codecov`) ainda.

## Fase 4 — IA e Desktop

- **Módulo de IA**: resumir eventos, detectar tendências/anomalias, explicar
  mudanças, responder perguntas em linguagem natural ("o que aconteceu nas
  últimas 24h?", "por que a latência aumentou?"), sempre fundamentado nos
  dados coletados (RAG sobre o histórico, não geração livre).
- ✅ **Dashboard Desktop** (`desktop/NetworkObservatory.Desktop`, WPF em vez
  de WinUI 3 — ver `desktop/README.md` para o motivo): consome a mesma API
  REST do web/CLI, lista de interfaces com status/MAC/throughput e alertas
  ativos, tema escuro, atualização ao vivo a cada 2s. Painéis desacoplados,
  widgets configuráveis e páginas de topologia/relatórios ficam no roadmap.
- ✅ **Multiusuário/autenticação** (`crates/store/src/auth.rs`, `POST
  /api/v1/auth/login|logout`, `GET /api/v1/auth/me`): usuários e sessões reais
  em SQLite, senha com hash argon2, sessão via cookie HttpOnly. Bootstrap
  automático de um usuário `admin` com senha aleatória no primeiro start
  (logada uma vez, padrão Grafana/Kibana). Web app inteiro fica atrás de tela
  de login (`AuthGate`). Enforcement no nível de rota REST é opt-in via
  `NETOBS_AUTH_REQUIRED=true` (desligado por padrão para não quebrar
  automações existentes que já consomem a API sem sessão — CLI, gRPC,
  integrações). Cadastro de múltiplos usuários hoje é via inserir diretamente
  na tabela `users` ou endpoint futuro `/auth/register`; filtros/pesquisa
  avançados na UI ficam no roadmap.

## Não planejado (fora de escopo permanente)

Qualquer funcionalidade de interceptação, modificação ou exploração de
tráfego de rede — o projeto é estritamente de observabilidade/diagnóstico.
