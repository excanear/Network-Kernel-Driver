# Modelo de Dados

Definido em `crates/collector-core/src/model.rs`, compartilhado por todas as
camadas (service, CLI, web via JSON).

## `InterfaceStats`

| Campo | Tipo | Origem Windows | Origem Linux |
|---|---|---|---|
| `index` | `u32` | `GetIfEntry2.InterfaceIndex` | índice de `/sys/class/net/<if>/ifindex` |
| `name` | `String` | nome amigável (`GetAdaptersAddresses`) | nome da interface (`ifname`) |
| `description` | `String` | `bDescr` | driver via symlink `/sys/class/net/<if>/device/driver`, senão `ifname` |
| `mac_address` | `String` | `PhysicalAddress` | `/sys/class/net/<if>/address` |
| `mtu` | `u32` | `GetIfEntry2.Mtu` | `/sys/class/net/<if>/mtu` |
| `oper_status` | `OperStatus` (enum) | `OperStatus` | `/sys/class/net/<if>/operstate` |
| `link_speed_bps` | `Option<u64>` | `TransmitLinkSpeed`/`ReceiveLinkSpeed` | `/sys/class/net/<if>/speed` (pode ser indisponível) |
| `duplex` | `Option<Duplex>` | melhor esforço / `None` | `/sys/class/net/<if>/duplex` |
| `if_type` | `String` | `IfType` mapeado (tabela IANA ifType) | `ARPHRD_*` de `/sys/class/net/<if>/type` |
| `ipv4_addresses` | `Vec<String>` | `GetAdaptersAddresses` | crate `if-addrs` |
| `ipv6_addresses` | `Vec<String>` | `GetAdaptersAddresses` | crate `if-addrs` |
| `rx_bytes`, `tx_bytes` | `u64` | contadores cumulativos | `/proc/net/dev` |
| `rx_packets`, `tx_packets` | `u64` | idem | idem |
| `rx_errors`, `tx_errors` | `u64` | `InErrors`/`OutErrors` | coluna errs de `/proc/net/dev` |
| `rx_drops`, `tx_drops` | `u64` | `InDiscards`/`OutDiscards` | coluna drop de `/proc/net/dev` |
| `rx_broadcast_packets` | `Option<u64>` | melhor esforço | não exposto de forma padronizada — `None` |
| `rx_multicast_packets` | `Option<u64>` | `InNUcastPkts` (aprox.) | coluna multicast de `/proc/net/dev` |
| `timestamp` | `DateTime<Utc>` | hora da amostragem (definida pelo coletor) | idem |
| `collector_source` | `CollectorSource` (enum) | `WindowsIpHelper` | `LinuxProcSys` — Fase 2 adiciona `KernelDriver` |

Todos os contadores são cumulativos desde o boot/reset do adaptador — taxas
(bytes/s) são calculadas no consumidor (service para o WS, ou no cliente web)
como delta entre duas amostras consecutivas.

## `Snapshot`

```rust
struct Snapshot {
    interfaces: Vec<InterfaceStats>,
    taken_at: DateTime<Utc>,
}
```

Uma amostragem completa de todas as interfaces em um instante — é a unidade
transportada tanto pelo REST (`GET /interfaces`) quanto pelo WebSocket
(mensagem por tick de poll).

## Trait `InterfaceCollector`

```rust
pub trait InterfaceCollector: Send + Sync {
    fn snapshot(&self) -> Result<Snapshot, CollectorError>;
    fn platform_name(&self) -> &'static str;
}
```

Contrato único implementado por `collector-windows` e `collector-linux` hoje;
a Fase 2 adiciona um backend que fala com o driver de modo kernel via
IOCTL/Netlink sem alterar este contrato — apenas populando os mesmos campos
com maior fidelidade (ex.: RSSI, temperatura, jitter/latência de baixo
nível quando disponíveis via driver).

## Persistência (`store`)

- **Memória**: `Arc<RwLock<HashMap<u32, VecDeque<InterfaceStats>>>>`, ring
  buffer limitado (~300 amostras/interface) para alimentar o gráfico ao vivo
  sem tocar disco.
- **SQLite** (`interface_stats`): amostra downsampled a cada ~10s, schema
  plano espelhando `InterfaceStats` + `if_index` — semente para os rollups
  multi-intervalo do roadmap (Fase 3).
- **Trait `HistoryStore`**: `insert_sample`, `query_range(if_index, from, to)`
  — permite trocar SQLite por Postgres/TimescaleDB (Fase 3) sem tocar em
  `service`/`cli`/`web`.
