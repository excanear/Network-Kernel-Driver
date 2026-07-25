# Desktop Dashboard (Fase N)

Implementado em **WPF** (.NET 8), não WinUI 3 — a stack WinUI 3 esbarrou num
componente do Visual Studio ausente nesta máquina (ferramentas de empacotamento
MSIX/Appx, `Microsoft.Build.Packaging.Pri.Tasks.dll`), cuja instalação exige
o VS Installer elevado (mesma limitação de acesso administrativo documentada
para a Fase L). WPF compila com o SDK do .NET puro, sem esse componente extra,
e entrega o mesmo resultado: um dashboard nativo Windows, tema escuro,
consumindo a mesma API REST que o CLI e o web usam.

## Rodar

```powershell
cd desktop/NetworkObservatory.Desktop
dotnet build
dotnet run
```

Requer `network-observatoryd` rodando em `http://127.0.0.1:7878` (padrão).

## Estrutura

- `App.xaml(.cs)` — bootstrap da aplicação WPF.
- `MainWindow.xaml(.cs)` — janela única: lista de interfaces (com indicador
  de status colorido, MAC, throughput) e lista de alertas ativos, atualizadas
  a cada 2s via `ApiClient`.
- `Models.cs` — DTOs espelhando `crates/collector-core`/`crates/alerts`
  (mesmo formato JSON que `web/src/lib/types.ts` consome).
- `ApiClient.cs` — cliente REST fino contra `GET /api/v1/interfaces` e
  `GET /api/v1/alerts`, provando a arquitetura "um backend, muitos
  frontends" (REST/WS/gRPC ← CLI, web, desktop).

## O que falta (roadmap)

- Múltiplos painéis/widgets configuráveis, drag-and-drop.
- Páginas de topologia (grafo) e relatórios.
- Migrar para WinUI 3 se/quando o componente MSIX/Appx do VS estiver
  disponível nesta máquina (documentado, ver nota acima).
