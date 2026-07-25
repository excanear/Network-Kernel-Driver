# Windows Kernel Driver (Fase 2 — não implementado ainda)

O driver KMDF/NDIS real ainda não foi implementado. A Fase 1 usa a IP Helper
API em modo usuário (`crates/collector-windows`) para dados reais sem
depender de um driver customizado.

Design completo, estrutura de fontes proposta, contrato IOCTL e processo de
build/assinatura: [`../../docs/phase2-kernel-driver-design.md`](../../docs/phase2-kernel-driver-design.md).
