# Windows Kernel Driver (Fase L — implementado, falta instalar)

Filtro NDIS 6.30 Lightweight Filter real (`NetObsFilter`), compilando e
linkando com sucesso contra o WDK (headers/libs em
`C:\Program Files (x86)\Windows Kits\10`), sem precisar da extensão VS do WDK
(que falhou ao instalar contra esta instância do VS 17.14 — build feito via
invocação direta de `cl.exe`/`link.exe`/`signtool.exe`, ver `build.ps1`).

## Build (não precisa de elevação)

```powershell
.\build.ps1
```

Compila `src\*.c`, linka `NetObsFilter.sys`, gera um certificado de
test-signing (`netobs-test-cert.pfx`) se ainda não existir, e assina o
driver com ele.

## Instalação (precisa de PowerShell elevado — Administrador)

```powershell
.\install-test-driver.ps1
```

1. Importa o certificado de teste em `LocalMachine\Root` e
   `LocalMachine\TrustedPublisher`.
2. Habilita test-signing via `bcdedit /set testsigning on` — **isso exige
   reiniciar o Windows** antes do driver poder carregar. O script para aqui
   na primeira execução e avisa para reiniciar.
3. Após o reboot, rode o script de novo para instalar via `pnputil`.

Este passo não foi executado nesta sessão porque o ambiente de automação não
tem uma sessão elevada disponível (mesma classe de limitação do `sudo` no
WSL2 documentada na Fase F/M) — precisa ser rodado manualmente por quem tiver
acesso de Administrador nesta máquina.

Assinatura de produção (certificado EV + submissão ao Windows Hardware Dev
Center) permanece fora do escopo automatizável, documentada em
[`../../docs/phase2-kernel-driver-design.md`](../../docs/phase2-kernel-driver-design.md).

## Estrutura

- `inc/filter.h`, `inc/ioctl_contract.h` — headers internos e contrato IOCTL compartilhado com `collector-kernel-windows` (futuro).
- `src/driver.c` — `DriverEntry`, registro do filtro NDIS.
- `src/filter.c` — Attach/Detach/Restart/Pause + passthrough puro de dados/status/OID.
- `src/telemetry.c` — consulta `OID_GEN_STATISTICS`/link state via `NdisFOidRequest`.
- `src/ioctl.c` — dispositivo de controle (`\\.\NetObsFilter`) para `IOCTL_NETOBS_GET_SNAPSHOT`.
- `NetworkObservatory.inf` — instalação do serviço/filtro.
