using DeadReckoningAdapter.Core.Exceptions;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Core.Kafka;
using DeadReckoningAdapter.Core.Processors;
using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Extension;
using DeadReckoningAdapter.Models;
using log4net;
using Microsoft.Extensions.Options;
using System.Diagnostics.CodeAnalysis;

namespace DeadReckoningAdapter
{
    [ExcludeFromCodeCoverage]
    public class Program
    {
        public static async Task Main(string[] args)
        {
            var builder = WebApplication.CreateBuilder(args);

            builder.Services.AddHealthChecks().AddCheck<CustomHealthCheckService>("Custom check");

            GlobalContext.Properties["log4net:HostName"] = builder.Configuration.GetRequiredSection("Log4Net:PAPERTRAIL_SYSTEM").Value;
            GlobalContext.Properties["log4net:Program"] = builder.Configuration.GetRequiredSection("Log4Net:PAPERTRAIL_PROGRAM").Value;
            if (Environment.GetEnvironmentVariable("IS_SLOT") == "true")
            {
                GlobalContext.Properties["log4net:LogPrefix"] = "[SLOT]";
            }
            else
            {
                GlobalContext.Properties["log4net:LogPrefix"] = string.Empty;
                builder.Services.AddHostedService<KafkaConsumer>();
            }
            builder.Logging.ClearProviders();
            builder.Logging.AddLog4Net(builder.Configuration.GetRequiredSection("Log4Net:Log4NetConfigFileName").Value);

            builder.Services.AddHttpClient<HttpApiClient>();
            builder.Services.AddScoped<ILocationCalculatorService, LocationCalculatorService>();
            builder.Services.AddSingleton<INewRelicService, NewRelicService>();
            builder.Services.Configure<KafkaSettings>(builder.Configuration.GetSection("KafkaSettings"));
            builder.Services.Configure<KeyVaultSettings>(builder.Configuration.GetSection("KeyVaultSettings"));
            builder.Services.Configure<RouteShapeSettings>(builder.Configuration.GetSection("RouteShapeSettings"));
            builder.Services.Configure<VehiclePositionSettings>(builder.Configuration.GetSection("VehiclePositionSettings"));
            builder.Services.AddSingleton<HealthState>();

            builder.Services.AddScoped<IMessageProcessor>(sp =>
            {
                var logger = sp.GetRequiredService<ILogger<MessageProcessor>>()
                    ?? throw new NoConfigurationException("logger is not configured");
                var redis = sp.GetRequiredService<IRedisCacheHelper>()
                    ?? throw new NoConfigurationException("redis is not configured");
                var locationCalculatorService = sp.GetRequiredService<ILocationCalculatorService>()
                    ?? throw new NoConfigurationException("locationCalculatorService is not configured");
                var routeShapeService = sp.GetRequiredService<IRouteShapeService>()
                    ?? throw new NoConfigurationException("routeShapeService is not configured");
                var kafkaProducer = sp.GetRequiredService<IKafkaProducer<VehiclePositionMessage>>()
                    ?? throw new NoConfigurationException("kafkaProducer is not configured");
                var vehiclePositionSettings = sp.GetRequiredService<IOptions<VehiclePositionSettings>>().Value;
                if (!vehiclePositionSettings.AllPropertiesNotNullOrEmpty())
                    throw new NoConfigurationException("vehiclePositionSettings is not correct");
                return new MessageProcessor(logger, redis, locationCalculatorService, routeShapeService, kafkaProducer, vehiclePositionSettings);
            });

            builder.Services.AddScoped<IRouteShapeService>(sp =>
            {
                var httpClient = sp.GetRequiredService<HttpApiClient>()
                    ?? throw new NoConfigurationException("httpClient is not configured");
                var routeShapeSettings = sp.GetRequiredService<IOptions<RouteShapeSettings>>().Value;
                if (!routeShapeSettings.AllPropertiesNotNullOrEmpty())
                    throw new NoConfigurationException("routeShapeSettings is not correct");
                var logger = sp.GetRequiredService<ILogger<RouteShapeService>>()
                    ?? throw new NoConfigurationException("logger is not configured");
                var redis = sp.GetRequiredService<IRedisCacheHelper>()
                    ?? throw new NoConfigurationException("redis is not configured");
                return new RouteShapeService(httpClient, routeShapeSettings, logger, redis);
            });

            builder.Services.AddSingleton<IKeyVaultHelper<ConfluentSecret>>(sp =>
            {
                var keyVaultSettings = sp.GetRequiredService<IOptions<KeyVaultSettings>>().Value;
                if (!keyVaultSettings.AllPropertiesNotNullOrEmpty())
                    throw new NoConfigurationException("keyVaultSettings is not correct");
                return new KeyVaultHelper<ConfluentSecret>(keyVaultSettings, [keyVaultSettings.ConfluentSecretName, keyVaultSettings.SchemaSecretName]);
            });

            builder.Services.AddSingleton<IKafkaProducer<VehiclePositionMessage>>(sp =>
            {
                var kafkaSettings = sp.GetRequiredService<IOptions<KafkaSettings>>().Value;
                if (!kafkaSettings.AllPropertiesNotNullOrEmpty())
                    throw new NoConfigurationException("kafkaSettings is not correct");
                var keyVaultHelper = sp.GetRequiredService<IKeyVaultHelper<ConfluentSecret>>()
                    ?? throw new NoConfigurationException("keyVaultHelper is not configured");
                var keyVaultSettings = sp.GetRequiredService<IOptions<KeyVaultSettings>>().Value;
                if (!keyVaultSettings.AllPropertiesNotNullOrEmpty())
                    throw new NoConfigurationException("keyVaultSettings is not correct");
                var logger = sp.GetRequiredService<ILogger<KafkaProducer<VehiclePositionMessage>>>()
                    ?? throw new NoConfigurationException("logger is not configured");
                var healthState = sp.GetRequiredService<HealthState>();
                var newRelicService = sp.GetRequiredService<INewRelicService>();
                var schemaRegistry = sp.GetRequiredService<ISchemaRegistry>();
                return new KafkaProducer<VehiclePositionMessage>(kafkaSettings, keyVaultSettings, logger, keyVaultHelper, schemaRegistry, newRelicService, healthState);
            });

            builder.Services.AddSingleton<ISchemaRegistry>(sp =>
            {
                var keyVaultSettings = sp.GetRequiredService<IOptions<KeyVaultSettings>>().Value;
                if (!keyVaultSettings.AllPropertiesNotNullOrEmpty())
                    throw new NoConfigurationException("keyVaultSettings is not correct");
                var keyVaultHelper = sp.GetRequiredService<IKeyVaultHelper<ConfluentSecret>>()
                    ?? throw new NoConfigurationException("keyVaultHelper is not configured");
                return new SchemaRegistry(keyVaultHelper, keyVaultSettings);
            });

            var redisConnectionString = builder.Configuration.GetConnectionString("Redis");
            if (string.IsNullOrWhiteSpace(redisConnectionString))
            {
                throw new NoConfigurationException("Redis Connection string is missing");
            }
            else
            {
                builder.Services.AddSingleton<IRedisCacheHelper>(new RedisCacheHelper(redisConnectionString));
            }

            var app = builder.Build();
            app.MapHealthChecks("/health");
            if (Environment.GetEnvironmentVariable("IS_SLOT") == "true")
            {

                var healthState = app.Services.GetRequiredService<HealthState>();
                healthState.ConsumerIsReady = true;
            }

            var logger = app.Services.GetRequiredService<ILogger<Program>>();
            var keyVaultHelper = app.Services.GetRequiredService<IKeyVaultHelper<ConfluentSecret>>();
            await keyVaultHelper.InitializeAsync();
            var schemaRegistry = app.Services.GetRequiredService<ISchemaRegistry>();
            schemaRegistry.SetSchemaRegistry();

            logger.LogInformation("Application is starting.");
            await app.StartAsync();
            await app.WaitForShutdownAsync();
        }
    }
}
