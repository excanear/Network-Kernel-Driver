# Plano de Conclusão — Network Observatory

> Gerado em 2026-10-09. Baseado no estado real do código + `docs/roadmap.md`.

## Estado atual
Muito mais completo do que o rótulo "EM DESENVOLVIMENTO" sugere. Fases 1–3 praticamente
concluídas: coleta real (APIs de SO), serviço Rust, REST/WebSocket/gRPC, CLI, dashboard web
ao vivo, health score, alertas, topologia, relatórios (CSV/JSON/HTML/MD/PDF), sistema de
plugins, auth argon2, auditoria, Postgres/TimescaleDB, serviço Windows/systemd, testes + CI.
Núcleo Rust compila limpo (`cargo check` validado nesta máquina).

## O que falta (itens não-✅ do roadmap)
1. **Fase 2 — crates de kernel-collector (próximo passo mais claro):**
   - `collector-kernel-windows`: fala com `\\.\NetObsFilter` via IOCTL (contrato em
     `driver/windows/inc/ioctl_contract.h`).
   - `collector-kernel-linux`: consome a família genetlink `netobs` (já funcional).
   - Os drivers já compilam; falta o lado Rust que os consome, atrás do trait `InterfaceCollector`.
2. **Instalação do driver** — `install-test-driver.ps1` (PowerShell elevado + reboot p/
   test-signing). **Passo manual do usuário** — automação não consegue (elevação+reboot).
3. **Fase 3 — histórico multi-intervalo (Fase F):** rollups automáticos (1m→365d) sobre
   `HistoryStore` + comparação entre períodos + timeline por evento no relatório.
4. **Fase 3 — sondagem ativa:** latência/jitter/RTT reais via ICMP raw sockets (hoje só o
   plugin `ping-latency-collector` via tempo de conexão TCP).
5. **Fase 3/D — alertas dependentes de topologia + notificação externa:** gateway offline,
   DNS lento, mudança de gateway/DNS; canais email/webhook/Slack.
6. **Fase 4 — módulo de IA:** RAG sobre o histórico (resumo de eventos, anomalias, Q&A em
   linguagem natural, sempre fundamentado nos dados — sem geração livre).
7. **Fase 4 — Desktop:** painéis desacoplados/configuráveis + páginas de topologia/relatórios no WPF.
8. **Auth:** endpoint `/auth/register` + gestão de usuários na UI.
9. **Qualidade:** cobertura formal (`cargo-tarpaulin`/codecov) no CI.

## Ferramentas / limitações
- Rust, Node e .NET disponíveis. Drivers exigem WDK + elevação/reboot (passo manual).
- Esforço: **médio**. Fatias independentes — priorizar 1, 3 e 5.
