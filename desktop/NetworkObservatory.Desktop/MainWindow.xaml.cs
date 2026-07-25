using System.Collections.ObjectModel;
using System.Windows;
using System.Windows.Threading;

namespace NetworkObservatory.Desktop;

public partial class MainWindow : Window
{
    private readonly ApiClient _api = new();
    private readonly DispatcherTimer _timer;
    private readonly ObservableCollection<InterfaceStats> _interfaces = new();
    private readonly ObservableCollection<Alert> _alerts = new();

    public MainWindow()
    {
        InitializeComponent();

        InterfacesList.ItemsSource = _interfaces;
        AlertsList.ItemsSource = _alerts;

        _timer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(2) };
        _timer.Tick += async (_, _) => await RefreshAsync();
        _timer.Start();

        _ = RefreshAsync();
    }

    private async Task RefreshAsync()
    {
        try
        {
            var snapshot = await _api.GetInterfacesAsync();
            var alerts = await _api.GetActiveAlertsAsync();

            ConnectionStatusText.Text = "live — network-observatoryd";

            if (snapshot is not null) SyncCollection(_interfaces, snapshot.Interfaces);
            if (alerts is not null) SyncCollection(_alerts, alerts);
        }
        catch (Exception ex)
        {
            ConnectionStatusText.Text = $"disconnected ({ex.Message})";
        }
    }

    private static void SyncCollection<T>(ObservableCollection<T> target, IReadOnlyList<T> source)
    {
        target.Clear();
        foreach (var item in source)
        {
            target.Add(item);
        }
    }
}
