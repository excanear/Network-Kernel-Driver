# Linux Kernel Module (Fase M — implementado e verificado carregando de verdade)

Módulo de kernel real (`netobs.ko`) que registra uma família Netlink genérica
(`netobs`) com um comando de leitura (`NETOBS_CMD_GET_SNAPSHOT`, dump de
estatísticas por interface) e um grupo multicast (`netobs_link`) para eventos
de mudança de link — via `register_netdevice_notifier`. Puramente observador:
nenhum hook de netfilter, nenhuma modificação de pacote ou de estado do
dispositivo (ver `include/netobs.h`).

**Verificado nesta sessão**: compilado contra os headers reais do kernel WSL2
(`6.18.33.2-microsoft-standard-WSL2`), carregado de verdade via `insmod`, com
a família aparecendo em `genl ctrl list` e retornando dados reais de
interface (`lo`, `eth0`) via uma consulta Netlink crua
(`scripts/netobs-genl-test.py`), e descarregado limpo via `rmmod`.

## Por que precisa de um kernel "preparado" em vez de `linux-headers-*`

O kernel do WSL2 é um build customizado da Microsoft, não um kernel de
distro empacotado — não existe pacote `linux-headers-6.18.33.2-microsoft-standard-WSL2`
no apt. O processo real usado:

```bash
git clone --branch linux-msft-wsl-6.18.33.2 --depth 1 \
  https://github.com/microsoft/WSL2-Linux-Kernel.git ~/wsl2-kernel
cd ~/wsl2-kernel
zcat /proc/config.gz > .config     # herda a config do kernel rodando
make olddefconfig
make modules_prepare               # gera o suficiente para compilar módulos externos
```

Isso **não** gera `Module.symvers` (só um build completo do kernel geraria),
então o link final precisa de `KBUILD_MODPOST_WARN=1` para transformar os
"unresolved symbol" (símbolos padrão como `printk`, `genlmsg_put`,
`__alloc_skb` — todos existentes no kernel real, só não verificáveis sem
`Module.symvers`) de erro em aviso. O `.ko` resultante carrega normalmente
via `insmod` no kernel de onde a config veio.

**Importante**: construa numa pasta sem espaços/acentos (ex.: `~/netobs-linux`,
não direto em `/mnt/c/.../OneDrive/...`) — o kbuild do kernel não lida bem
com `M=<caminho com espaço>`.

## Build e teste

```bash
# 1. Preparar a árvore do kernel (uma vez só, ~5-10 min)
#    ver comandos acima

# 2. Copiar este diretório para um caminho sem espaços dentro do WSL
cp -r driver/linux ~/netobs-linux

# 3. Compilar
cd ~/netobs-linux
make KDIR=~/wsl2-kernel KBUILD_MODPOST_WARN=1

# 4. Carregar e verificar
sudo insmod netobs.ko
sudo genl ctrl list | grep -A8 netobs
sudo dmesg | tail -5

# 5. Consultar dados reais (sem dependências externas)
python3 scripts/netobs-genl-test.py

# 6. Descarregar
sudo rmmod netobs
```

## Estrutura

- `include/netobs_uapi.h` — contrato Netlink compartilhado (comandos/atributos), espelha `driver/windows/inc/ioctl_contract.h`.
- `include/netobs.h` — header interno do módulo.
- `src/netobs_main.c` — `module_init`/`exit`, notifier de eventos de link.
- `src/netobs_netlink.c` — registro da família genetlink, handler de dump do `GET_SNAPSHOT`.
- `src/netobs_collect.c` — leitura de `dev_get_stats`/carrier state/ethtool por interface.
- `Makefile`, `build.sh` — build out-of-tree contra uma árvore de kernel preparada.
- `dkms.conf` — empacotamento DKMS para sobreviver a upgrades de kernel (produção).

## O que falta para produção

- Build de kernel completo (não só `modules_prepare`) para gerar
  `Module.symvers` de verdade e eliminar os avisos de símbolo.
- Empacotamento DKMS testado (`dkms.conf` já presente, não testado nesta sessão).
- `collector-kernel-linux` (crate Rust): backend que fala com este módulo via
  Netlink, análogo ao IOCTL do lado Windows — ainda não implementado
  (o backend real hoje continua sendo `collector-linux` via procfs/sysfs).
