using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Core.Kafka;
using DeadReckoningAdapter.Core.Processors;
using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Models;
using Microsoft.Extensions.Logging;
using Moq;

namespace DeadReckoningAdapter.Tests.Core.Processors
{
    public class MessageProcessorTests
    {
        private readonly Mock<IRouteShapeService> _mockRouteShapeService;
        private readonly Mock<ILogger<MessageProcessor>> _mockLogger;
        private readonly Mock<IRedisCacheHelper> _mockRedisCacheHelper;
        private readonly Mock<ILocationCalculatorService> _mockLocationCalculatorService;
        private readonly Mock<IKafkaProducer<VehiclePositionMessage>> _kafkaProducer;
        private readonly VehiclePositionSettings _vehiclePositionSettings;
        private readonly MessageProcessor _messageProcessor;

        public MessageProcessorTests()
        {
            _mockRouteShapeService = new Mock<IRouteShapeService>();
            _mockLogger = new Mock<ILogger<MessageProcessor>>();
            _mockRedisCacheHelper = new Mock<IRedisCacheHelper>();
            _mockLocationCalculatorService = new Mock<ILocationCalculatorService>();
            _kafkaProducer = new Mock<IKafkaProducer<VehiclePositionMessage>>();
            _vehiclePositionSettings = new VehiclePositionSettings { VehiclePositionRedisKey = "test-vp-redis-key", Ttl = 3600 };
            _messageProcessor = new MessageProcessor(
                _mockLogger.Object,
                _mockRedisCacheHelper.Object,
                _mockLocationCalculatorService.Object,
                _mockRouteShapeService.Object,
                _kafkaProducer.Object,
                _vehiclePositionSettings);
        }

        [Fact]
        public async Task ProcessVPMessage_VPIsValid_ShouldPutValueInRedisCache()
        {
            // Arrange
            var vpMessage = new VehiclePositionMessage()
            {
                Id = "testId",
                VehiclePosition = new VehiclePosition()
                {
                    Position = new Position { Bearing = 1, Odometer = 12345, Speed = 60, Latitude = 174.55555, Longitude = 36.3636 },
                    Vehicle = new Vehicle { Id = "vehicleId", Label = "vehicleLabel", LicensePlate = "plateNumber" },
                    Trip = new Trip { TripId = "tripId", RouteId = "routeId", DirectionId = 1, StartDate = "11/05/2024", StartTime = "10:00", ScheduleRelationship = "rel" },
                    OccupancyStatus = "FULL",
                    Timestamp = "12345"
                }
            };

            // Act
            await _messageProcessor.ProcessVPMessage(vpMessage);

            // Assert
            _mockRedisCacheHelper.Verify(redis =>
                redis.SetCacheAsync(
                    It.Is<string>(s => s == $"{_vehiclePositionSettings.VehiclePositionRedisKey}:tripId:vehicleId"),
                    It.Is<VehiclePositionMessage>(value => value == vpMessage),
                    It.Is<TimeSpan>(ts => ts == TimeSpan.FromHours(1))),
                    Times.Once);
        }

        [Fact]
        public async Task ProcessDRMessage_VPIsExisting_ShouldPublishNewVpMessage()
        {
            // Arrange
            var vpMessage = new VehiclePositionMessage()
            {
                Id = "testId",
                VehiclePosition = new VehiclePosition()
                {
                    Position = new Position { Bearing = 1, Odometer = 12345, Speed = 60, Latitude = 174.55555, Longitude = 36.3636 },
                    Vehicle = new Vehicle { Id = "vehicleId", Label = "vehicleLabel", LicensePlate = "plateNumber" },
                    Trip = new Trip { TripId = "tripId-999", RouteId = "routeId", DirectionId = 1, StartDate = "11/05/2024", StartTime = "10:00", ScheduleRelationship = "rel" },
                    OccupancyStatus = "FULL",
                    Timestamp = "12345"
                }
            };
            var drMessage = new DeadReckoningMessage()
            {
                Id = "id123",
                Position = new PositionDR() { Odometer = 55555 },
                Trip = new Trip() { TripId = "tripId-999", RouteId = "routeId" },
                Vehicle = new VehicleDR() { Id = "vehicleId" },
                ReceivedAt = 123456
            };
            var routeShape = new List<PathPoint>()
            {
                new PathPoint {
                    Longitude = 174.66678,
                    Latitude = -36.39801,
                },
                new PathPoint {
                    Longitude = 174.66675,
                    Latitude = -36.39806,
                },
                new PathPoint {
                    Longitude = 174.66685,
                    Latitude = -36.3981,
                },
                new PathPoint {
                    Longitude = 174.66629,
                    Latitude = -36.39894,
                },
                new PathPoint {
                    Longitude = 174.66605,
                    Latitude = -36.39927,
                },
            };
            var location = new Location() { Latitude = 174, Longitude = 36 };
            _mockRedisCacheHelper.Setup(redis => redis.GetCacheAsync<VehiclePositionMessage>(It.IsAny<string>())).ReturnsAsync(vpMessage);
            _mockRouteShapeService.Setup(routeShape => routeShape.GetRouteShapeByRouteId(It.IsAny<string>(), It.IsAny<string>())).ReturnsAsync(routeShape);
            _mockLocationCalculatorService.Setup(locationCalculator =>
                locationCalculator.FindVP(It.IsAny<DeadReckoningMessage>(), It.IsAny<VehiclePositionMessage>(), It.IsAny<List<PathPoint>>())).Returns(location);

            // Act
            await _messageProcessor.ProcessDRMessage(drMessage);

            // Assert
            _kafkaProducer.Verify(producer =>
                producer.ProduceMessage(
                    It.Is<string>(s => s == "testId"),
                    It.Is<VehiclePositionMessage>(value => value.VehiclePosition.Position.Longitude == location.Longitude &&
                        value.VehiclePosition.Position.Latitude == location.Latitude)),
                    Times.Once);
        }

        [Fact]
        public async Task ProcessDRMessage_VPIsntExisting_ShouldPublishNewVpMessageWithRouteStartLocationAsPostition()
        {
            var drMessage = new DeadReckoningMessage()
            {
                Id = "id123",
                Position = new PositionDR() { Odometer = 55555 },
                Trip = new Trip() { TripId = "tripId-999", RouteId = "routeId" },
                Vehicle = new VehicleDR() { Id = "vehicleId" },
                ReceivedAt = 123456
            };
            var routeShape = new List<PathPoint>()
            {
                new PathPoint {
                    Longitude = 174.66678,
                    Latitude = -36.39801,
                },
                new PathPoint {
                    Longitude = 174.66675,
                    Latitude = -36.39806,
                },
                new PathPoint {
                    Longitude = 174.66685,
                    Latitude = -36.3981,
                },
                new PathPoint {
                    Longitude = 174.66629,
                    Latitude = -36.39894,
                },
                new PathPoint {
                    Longitude = 174.66605,
                    Latitude = -36.39927,
                },
            };

            _mockRedisCacheHelper.Setup(redis => redis.GetCacheAsync<VehiclePositionMessage>(It.IsAny<string>())).ReturnsAsync((VehiclePositionMessage)null);
            _mockRouteShapeService.Setup(routeShape => routeShape.GetRouteShapeByRouteId(It.IsAny<string>(), It.IsAny<string>())).ReturnsAsync(routeShape);

            // Act
            await _messageProcessor.ProcessDRMessage(drMessage);

            // Assert
            _kafkaProducer.Verify(producer =>
                producer.ProduceMessage(
                    It.Is<string>(s => s == "vehicleId"),
                    It.Is<VehiclePositionMessage>(value => value.VehiclePosition.Position.Longitude == routeShape[0].Longitude &&
                        value.VehiclePosition.Position.Latitude == routeShape[0].Latitude &&
                        value.VehiclePosition.Position.Odometer == drMessage.Position.Odometer)),
                    Times.Once);
        }
    }
}
