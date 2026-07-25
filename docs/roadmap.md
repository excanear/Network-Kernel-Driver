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
- **Topologia**: detecção automática de NIC, gateway, DNS, VPN, interfaces
  virtuais (Hyper-V, Docker, WSL, VMware, VirtualBox), grafo interativo
  (Cytoscape.js) na Camada 6.
- **Health score**: disponibilidade, estabilidade, perda de pacotes, mudanças
  de link, qualidade, temperatura (quando exposta pela Fase 2), erros,
  latência, jitter — agregados num score único por interface.
- **Motor de alertas**: regras configuráveis (packet loss > X, latência > X,
  link down, interface/gateway offline, DNS lento, erro CRC, mudança de
  IP/gateway/DNS, velocidade negociada reduzida, temperatura elevada),
  canais de notificação plugáveis.
- **Relatórios**: geração automática em PDF/CSV/JSON/HTML/Markdown (resumo
  executivo, timeline, eventos, métricas, gráficos, health, alertas,
  conclusões) — `network export` na CLI sai do estado de stub.
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
