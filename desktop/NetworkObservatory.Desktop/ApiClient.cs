using System.Net.Http;
using System.Net.Http.Json;

namespace NetworkObservatory.Desktop;

/// Thin REST client against network-observatoryd (crates/service) — the same
/// API the web dashboard (web/src/lib/api.ts) and CLI (crates/cli) consume,
/// proving the "one backend, many frontends" architecture from
/// docs/architecture.md.
public class ApiClient
{
    private readonly HttpClient _http;

    public ApiClient(string baseUrl = "http://127.0.0.1:7878/api/v1")
    {
        _http = new HttpClient { BaseAddress = new Uri(baseUrl + "/") };
    }

    public async Task<Snapshot?> GetInterfacesAsync(CancellationToken ct = default) =>
        await _http.GetFromJsonAsync<Snapshot>("interfaces", ct);

    public async Task<List<HealthScore>?> GetHealthAsync(CancellationToken ct = default) =>
        await _http.GetFromJsonAsync<List<HealthScore>>("health", ct);

    public async Task<List<Alert>?> GetActiveAlertsAsync(CancellationToken ct = default) =>
        await _http.GetFromJsonAsync<List<Alert>>("alerts", ct);
}
