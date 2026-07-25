# Fase 2 — Driver de Modo Kernel (design)

Este documento descreve o driver de modo kernel real que substituirá/
complementará os backends de modo usuário da Fase 1, adicionando dados que só
o kernel/firmware expõe de forma confiável (eventos de mudança de link em
tempo real de baixa latência, contadores de hardware não expostos por
`GetIfEntry2`/`procfs`, RSSI/temperatura quando o driver da NIC os expõe via
OID/ethtool, etc.). Não implementado na Fase 1 — ver justificativa em
[`architecture.md`](architecture.md#por-que-a-camada-1-tem-dois-estágios).

## Windows — driver KMDF/NDIS

### Papel

Filtro NDIS leve (**Lightweight Filter driver**, `NDIS_FILTER_DRIVER`) sobre
KMDF. Não intercepta nem modifica pacotes — apenas observa estatísticas
(`OID` queries), status de indicação de link (`NDIS_STATUS_LINK_STATE`) e
expõe um dispositivo de controle para IOCTL.

### Estrutura de fontes (`driver/windows/`)

```
driver/windows/
├── src/
│   ├── driver.c            # DriverEntry, unload
│   ├── filter.c            # attach/detach do filtro NDIS, OID handling
│   ├── ioctl.c              # dispositivo de controle, dispatch de IOCTL
│   └── telemetry.c          # coleta de contadores/eventos, buffer circular kernel-side
├── inc/
│   └── ioctl_contract.h     # códigos IOCTL e structs compartilhadas com o service
├── NetworkObservatory.inf   # instalação (Plug and Play / control device)
├── NetworkObservatory.vcxproj
└── README.md
```

### Contrato IOCTL (`ioctl_contract.h`, compartilhado via header com o `service`)

- `IOCTL_NETOBS_GET_SNAPSHOT` — retorna um buffer com o mesmo shape lógico de
  `Snapshot`/`InterfaceStats` (Fase 1), populado a partir de dados do
  driver em vez da IP Helper API.
- `IOCTL_NETOBS_SUBSCRIBE_EVENTS` — habilita notificação assíncrona
  (via `IRP` pendente ou evento nomeado) de mudanças de link.
- Validação: todo buffer de entrada é validado por tamanho e alinhamento antes
  do uso (`METHOD_BUFFERED`); nenhuma ponteiro de user-mode é dereferenciado
  diretamente no kernel.

### Build & assinatura

1. WDK + Visual Studio (Driver Development workload).
2. Build local: test-signing habilitado (`bcdedit /set testsigning on`) +
   certificado de teste autoassinado para iteração.
3. Produção: assinatura EV + submissão ao Windows Hardware Dev Center
   (attestation signing) para carregar sem test-signing habilitado.
4. `service` passa a detectar o driver instalado (via `SetupDi*` ou tentativa
   de `CreateFile` no dispositivo de controle) e usa o backend `KernelDriver`
   automaticamente; se ausente, cai de volta para `collector-windows`
   (fallback gracioso, mesmo trait `InterfaceCollector`).

## Linux — módulo de kernel + Netlink

### Papel

Módulo de kernel carregável (LKM) que registra uma família Netlink genérica
(`genetlink`) dedicada (`NETOBS_GENL_FAMILY`), publicando snapshots e eventos
de link sob demanda/subscrição — sem hooks em `netfilter`, sem modificação de
pacotes.

### Estrutura de fontes (`driver/linux/`)

```
driver/linux/
├── src/
│   ├── netobs_main.c        # module_init/exit, registro da família genetlink
│   ├── netobs_netlink.c     # handlers de comando (GET_SNAPSHOT, SUBSCRIBE)
│   └── netobs_collect.c     # leitura de struct net_device_stats / notifiers
├── include/
│   └── netobs_uapi.h        # enums/attrs Netlink compartilhados com o service (bindgen)
├── Makefile                 # kbuild padrão (out-of-tree module)
├── dkms.conf                # empacotamento DKMS para sobreviver a upgrades de kernel
└── README.md
```

### Contrato Netlink (`netobs_uapi.h`)

- Comandos: `NETOBS_CMD_GET_SNAPSHOT`, `NETOBS_CMD_SUBSCRIBE_LINK_EVENTS`.
- Atributos mapeando 1:1 os campos de `InterfaceStats` (mesmo raciocínio do
  lado Windows — o `service` consome o mesmo modelo lógico independente do
  backend).
- Eventos de link via `register_netdevice_notifier` no kernel, repassados
  como multicast Netlink para o `service` inscrito.

### Build & carregamento

1. `make -C /lib/modules/$(uname -r)/build M=$(pwd) modules` — requer headers
   do kernel em execução (`linux-headers-$(uname -r)` ou equivalente).
2. Empacotado via DKMS para rebuild automático em updates de kernel.
3. `insmod`/`modprobe` requer `CAP_SYS_MODULE`; o `service` roda sem esse
   privilégio — apenas o passo de instalação do módulo é privilegiado.
4. Mesmo fallback gracioso: se o módulo não estiver carregado, `service` usa
   `collector-linux` (procfs/sysfs).

## Impacto no `service` (Fase 2)

- Novo backend `collector-kernel` (crate), implementando `InterfaceCollector`,
  detectado em runtime; ordem de preferência configurável
  (`KernelDriver` > `IpHelper`/`ProcSys`).
- `CollectorSource::KernelDriver` já reservado no enum desde a Fase 1
  (ver [`data-model.md`](data-model.md)) — nenhuma mudança de schema.
- Nenhuma mudança necessária em `cli` ou `web`: consomem o mesmo
  `Snapshot`/`InterfaceStats` independentemente da origem.

## Segurança

- Superfície de ataque do IOCTL/Netlink limitada a leitura de estatísticas —
  sem caminho de escrita que afete o tráfego.
- Todo buffer cruzando o limite kernel/user é validado por tamanho antes do
  uso; sem confiança implícita em ponteiros de user-mode.
- Logs de carregamento/descarregamento do driver e de falhas de validação
  são auditáveis (ETW no Windows, `dmesg`/`ftrace` no Linux), agregados pelo
  `service` no sistema de logs da Fase 3.
