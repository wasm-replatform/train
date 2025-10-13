using DeadReckoningAdapter.Models;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using System.Diagnostics.CodeAnalysis;

namespace DeadReckoningAdapter.Core.Services;

[ExcludeFromCodeCoverage]
public class CustomHealthCheckService : IHealthCheck
{
    private readonly HealthState _healthState;

    public CustomHealthCheckService(HealthState healthState)
    {
        _healthState = healthState;
    }

    public Task<HealthCheckResult> CheckHealthAsync(HealthCheckContext context, CancellationToken cancellationToken = default)
    {
        if (_healthState.ConsumerIsReady && _healthState.ProducerIsReady)
        {
            return Task.FromResult(HealthCheckResult.Healthy("The service is running and ready."));
        }
        else
        {
            return Task.FromResult(HealthCheckResult.Unhealthy("The service is not ready."));
        }
    }
}
