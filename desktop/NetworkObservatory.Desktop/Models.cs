using System.Text.Json.Serialization;
using System.Windows.Media;

namespace NetworkObservatory.Desktop;

// Mirrors crates/collector-core/src/model.rs — field names match the Rust
// struct's serde (snake_case) output, same wire format the web app consumes.
public class InterfaceStats
{
    [JsonPropertyName("index")] public uint Index { get; set; }
    [JsonPropertyName("name")] public string Name { get; set; } = "";
    [JsonPropertyName("description")] public string Description { get; set; } = "";
    [JsonPropertyName("mac_address")] public string MacAddress { get; set; } = "";
    [JsonPropertyName("mtu")] public uint Mtu { get; set; }
    [JsonPropertyName("oper_status")] public string OperStatus { get; set; } = "Unknown";
    [JsonPropertyName("link_speed_bps")] public ulong? LinkSpeedBps { get; set; }
    [JsonPropertyName("ipv4_addresses")] public List<string> Ipv4Addresses { get; set; } = new();
    [JsonPropertyName("ipv6_addresses")] public List<string> Ipv6Addresses { get; set; } = new();
    [JsonPropertyName("rx_bytes")] public ulong RxBytes { get; set; }
    [JsonPropertyName("tx_bytes")] public ulong TxBytes { get; set; }

    public SolidColorBrush StatusBrush => new(OperStatus switch
    {
        "Up" => Colors.LimeGreen,
        "Down" or "NotPresent" => Colors.IndianRed,
        "LowerLayerDown" => Colors.Orange,
        _ => Colors.Gray,
    });

    public string ThroughputSummary => $"RX {HumanBytes(RxBytes)}  ·  TX {HumanBytes(TxBytes)}";

    private static string HumanBytes(ulong bytes)
    {
        string[] units = { "B", "KB", "MB", "GB", "TB" };
        double value = bytes;
        int i = 0;
        while (value >= 1024 && i < units.Length - 1) { value /= 1024; i++; }
        return $"{value:0.0}{units[i]}";
    }
}

public class Snapshot
{
    [JsonPropertyName("interfaces")] public List<InterfaceStats> Interfaces { get; set; } = new();
    [JsonPropertyName("taken_at")] public DateTimeOffset TakenAt { get; set; }
}

public class HealthScore
{
    [JsonPropertyName("if_index")] public uint IfIndex { get; set; }
    [JsonPropertyName("score")] public double Score { get; set; }
    [JsonPropertyName("availability_pct")] public double AvailabilityPct { get; set; }
    [JsonPropertyName("packet_loss_pct")] public double PacketLossPct { get; set; }
}

public class Alert
{
    [JsonPropertyName("id")] public string Id { get; set; } = "";
    [JsonPropertyName("if_name")] public string IfName { get; set; } = "";
    [JsonPropertyName("kind")] public string Kind { get; set; } = "";
    [JsonPropertyName("severity")] public string Severity { get; set; } = "";
    [JsonPropertyName("message")] public string Message { get; set; } = "";
    [JsonPropertyName("triggered_at")] public DateTimeOffset TriggeredAt { get; set; }

    public SolidColorBrush SeverityBrush => new(Severity switch
    {
        "Critical" => Colors.IndianRed,
        "Warning" => Colors.Orange,
        _ => Colors.LimeGreen,
    });
}
