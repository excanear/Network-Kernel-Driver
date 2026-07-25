# Linux Kernel Module (Fase 2 — não implementado ainda)

O módulo de kernel + família Netlink real ainda não foi implementado. A
Fase 1 usa `/proc/net/dev` e `/sys/class/net/*` em modo usuário
(`crates/collector-linux`) para dados reais sem depender de um módulo
customizado.

Design completo, estrutura de fontes proposta, contrato Netlink e processo de
build/carregamento: [`../../docs/phase2-kernel-driver-design.md`](../../docs/phase2-kernel-driver-design.md).
