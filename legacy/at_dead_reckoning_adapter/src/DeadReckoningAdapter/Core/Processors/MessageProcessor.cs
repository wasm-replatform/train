using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Core.Kafka;
using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Models;
using Newtonsoft.Json;

namespace DeadReckoningAdapter.Core.Processors
{
    public interface IMessageProcessor
    {
        public Task ProcessVPMessage(VehiclePositionMessage value);
        public Task ProcessDRMessage(DeadReckoningMessage value);
    }

    public class MessageProcessor : IMessageProcessor
    {
        private readonly ILogger<MessageProcessor> _logger;
        private readonly IRedisCacheHelper _redisCache;
        private readonly ILocationCalculatorService _locationCalculatorService;
        private readonly IRouteShapeService _routeShapeService;
        private readonly IKafkaProducer<VehiclePositionMessage> _kafkaProducer;
        private readonly VehiclePositionSettings _vehiclePositionSettings;

        public MessageProcessor(ILogger<MessageProcessor> logger,
            IRedisCacheHelper redisCache,
            ILocationCalculatorService locationCalculatorService,
            IRouteShapeService routeShapeService,
            IKafkaProducer<VehiclePositionMessage> kafkaProducer,
            VehiclePositionSettings vehiclePositionSettings)
        {
            _logger = logger;
            _redisCache = redisCache;
            _locationCalculatorService = locationCalculatorService;
            _routeShapeService = routeShapeService;
            _kafkaProducer = kafkaProducer;
            _vehiclePositionSettings = vehiclePositionSettings;
        }

        public async Task ProcessVPMessage(VehiclePositionMessage value)
        {
            if (!String.IsNullOrEmpty(value.VehiclePosition.Trip?.TripId) && !String.IsNullOrEmpty(value.VehiclePosition.Vehicle.Id) && value.VehiclePosition.Position.Odometer is not null)
            {
                await _redisCache.SetCacheAsync(
                    $"{_vehiclePositionSettings.VehiclePositionRedisKey}:{value.VehiclePosition.Trip.TripId}:{value.VehiclePosition.Vehicle.Id}",
                    value,
                    TimeSpan.FromSeconds(_vehiclePositionSettings.Ttl));
            }
        }

        public async Task ProcessDRMessage(DeadReckoningMessage value)
        {
            _logger.LogDebug($"Handle message from DR topic: {JsonConvert.SerializeObject(value)}");
            var cachedVP = await _redisCache.GetCacheAsync<VehiclePositionMessage>($"{_vehiclePositionSettings.VehiclePositionRedisKey}:{value.Trip.TripId}:{value.Vehicle.Id}");
            if (!String.IsNullOrEmpty(value.Trip.TripId) &&
                !String.IsNullOrEmpty(value.Vehicle.Id) &&
                cachedVP is not null &&
                cachedVP.VehiclePosition.Trip is not null)
            {
                var routeShape = await _routeShapeService.GetRouteShapeByRouteId(cachedVP.VehiclePosition.Trip.RouteId,
                                                                                 cachedVP.VehiclePosition.Trip.TripId.Split("-")[1]);
                var location = _locationCalculatorService.FindVP(value, cachedVP, routeShape);

                if (location is not null)
                {
                    _logger.LogDebug($"previous VP message : {JsonConvert.SerializeObject(cachedVP)}");
                    cachedVP.VehiclePosition.Position.Longitude = location.Longitude;
                    cachedVP.VehiclePosition.Position.Latitude = location.Latitude;
                    cachedVP.VehiclePosition.Position.Odometer = value.Position.Odometer;
                    cachedVP.VehiclePosition.Timestamp = new DateTimeOffset(DateTime.UtcNow).ToUnixTimeSeconds().ToString();
                    cachedVP.VehiclePosition.OccupancyStatus = cachedVP.VehiclePosition.OccupancyStatus ?? "";
                    cachedVP.VehiclePosition.Vehicle.LicensePlate = cachedVP.VehiclePosition.Vehicle.LicensePlate ?? "";
                    _logger.LogDebug($"Publish new VP message : {JsonConvert.SerializeObject(cachedVP)}");
                    await _kafkaProducer.ProduceMessage(cachedVP.Id, cachedVP);
                }
            }
            else if (cachedVP is null &&
                !String.IsNullOrEmpty(value.Trip.TripId) &&
                !String.IsNullOrEmpty(value.Vehicle.Id) &&
                !String.IsNullOrEmpty(value.Trip.RouteId))
            {
                var routeShape = await _routeShapeService.GetRouteShapeByRouteId(value.Trip.RouteId,
                                                                                 value.Trip.TripId.Split("-")[1]);
                if (routeShape.Any())
                {
                    _logger.LogDebug($"previous VP message : not exist");
                    var vp = new VehiclePositionMessage
                    {
                        Id = value.Vehicle.Id,
                        VehiclePosition = new VehiclePosition
                        {
                            Vehicle = new Vehicle { Id = value.Vehicle.Id, Label = "", LicensePlate = "" },
                            Position = new Models.Position
                            {
                                Longitude = routeShape[0].Longitude,
                                Latitude = routeShape[0].Latitude,
                                Odometer = value.Position.Odometer,
                                Speed = 0,
                                Bearing = 0
                            },
                            Trip = value.Trip,
                            Timestamp = new DateTimeOffset(DateTime.UtcNow).ToUnixTimeSeconds().ToString(),
                            OccupancyStatus = "",
                        }

                    };
                    _logger.LogDebug($"Publish new VP message : {JsonConvert.SerializeObject(vp)}");
                    await _kafkaProducer.ProduceMessage(vp.Id, vp);
                }
            }
        }
    }
}
