# Especificação de API

## REST — `/api/v1`

Base local de desenvolvimento: `http://localhost:7878/api/v1`

| Método | Rota | Descrição |
|---|---|---|
| GET | `/status` | Saúde do serviço: uptime, backend do coletor, intervalo de poll, nº de interfaces |
| GET | `/interfaces` | Snapshot atual de todas as interfaces |
| GET | `/interfaces/:index` | Snapshot atual de uma interface |
| GET | `/interfaces/:index/history?from=&to=&limit=` | Amostras históricas (SQLite) |
| GET | `/version` | Metadados de versão/build do serviço |

Respostas em JSON, serializando `Snapshot`/`InterfaceStats` (ver
[`data-model.md`](data-model.md)) via `serde`.

### `GET /status` — exemplo de resposta

```json
{
  "uptime_seconds": 128,
  "collector_backend": "WindowsIpHelper",
  "poll_interval_ms": 1000,
  "interface_count": 6
}
```

## WebSocket

`GET /api/v1/ws/interfaces` — upgrade para WebSocket; o servidor envia uma
mensagem JSON (`Snapshot`) a cada tick do loop de poll (mesmo ciclo que
alimenta o REST — fonte única de coleta, sem chamadas duplicadas ao
coletor). Múltiplos clientes compartilham o mesmo broadcast channel.

## gRPC (documentado, não implementado na Fase 1)

Contrato `.proto` alvo (mesmo shape do modelo REST/WS), para implementação na
Fase 3 quando houver consumidores que se beneficiem de streaming bidirecional
tipado (ex.: plugins, integrações externas):

```protobuf
syntax = "proto3";
package network_observatory.v1;

service InterfaceService {
  rpc GetSnapshot(SnapshotRequest) returns (Snapshot);
  rpc StreamSnapshots(SnapshotRequest) returns (stream Snapshot);
  rpc GetHistory(HistoryRequest) returns (HistoryResponse);
}

message SnapshotRequest {}

message Snapshot {
  repeated InterfaceStats interfaces = 1;
  int64 taken_at_unix_ms = 2;
}

message InterfaceStats {
  uint32 index = 1;
  string name = 2;
  string description = 3;
  string mac_address = 4;
  uint32 mtu = 5;
  string oper_status = 6;
  optional uint64 link_speed_bps = 7;
  optional string duplex = 8;
  string if_type = 9;
  repeated string ipv4_addresses = 10;
  repeated string ipv6_addresses = 11;
  uint64 rx_bytes = 12;
  uint64 tx_bytes = 13;
  uint64 rx_packets = 14;
  uint64 tx_packets = 15;
  uint64 rx_errors = 16;
  uint64 tx_errors = 17;
  uint64 rx_drops = 18;
  uint64 tx_drops = 19;
  optional uint64 rx_broadcast_packets = 20;
  optional uint64 rx_multicast_packets = 21;
  int64 timestamp_unix_ms = 22;
  string collector_source = 23;
}

message HistoryRequest {
  uint32 if_index = 1;
  int64 from_unix_ms = 2;
  int64 to_unix_ms = 3;
  uint32 limit = 4;
}

message HistoryResponse {
  repeated InterfaceStats samples = 1;
}
```

## CLI (`network`)

Estilo `nvidia-smi`/`kubectl`/`docker`/`git`: subcomandos verbais, tabela por
padrão, `--json` para automação.

**Implementados na Fase 1:**

| Comando | Descrição |
|---|---|
| `network status` | Chama `/status`, imprime resumo de saúde do serviço |
| `network interfaces [--json]` | Chama `/interfaces`, tabela ou JSON |
| `network monitor [--interval 1s] [--interface <nome>]` | Conecta ao WS, tabela ao vivo no terminal |
| `network history --interface <n> [--from] [--to] [--limit]` | Chama `/interfaces/:index/history` |

**Stubados (parseados, imprimem "não implementado ainda" com referência ao roadmap):**

`network topology`, `network export`, `network alerts`, `network statistics`.
