# Roadmap

## Fase 1 — Fatia vertical funcional (concluída)

Coleta real (APIs de SO) → serviço Rust → REST + WebSocket → CLI → dashboard
web ao vivo. Ver [`phase1-slice-design.md`](phase1-slice-design.md).

## Fase 2 — Driver de modo kernel real

- Windows: driver KMDF/NDIS assinado.
- Linux: módulo de kernel + família Netlink.
- Backend `collector-kernel` plugável atrás do mesmo trait `InterfaceCollector`.

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
- **Sistema de plugins**: coletores, dashboards, gráficos, alertas,
  relatórios e integrações adicionais carregáveis sem alterar o core.
- **Logging estruturado com rotação** e trilha de auditoria completa.
- **Banco de dados**: suporte PostgreSQL/TimescaleDB como alternativa ao
  SQLite para séries temporais em escala.

## Fase 4 — IA e Desktop

- **Módulo de IA**: resumir eventos, detectar tendências/anomalias, explicar
  mudanças, responder perguntas em linguagem natural ("o que aconteceu nas
  últimas 24h?", "por que a latência aumentou?"), sempre fundamentado nos
  dados coletados (RAG sobre o histórico, não geração livre).
- **Dashboard Desktop** (WPF/WinUI): mesma API REST/WS/gRPC do web, UI
  premium inspirada em Grafana/Datadog/GlassWire/Intel Performance Analyzer,
  painéis desacoplados e widgets configuráveis.
- **Multiusuário** no dashboard web, com filtros/pesquisa/exportação
  avançados.

## Não planejado (fora de escopo permanente)

Qualquer funcionalidade de interceptação, modificação ou exploração de
tráfego de rede — o projeto é estritamente de observabilidade/diagnóstico.
