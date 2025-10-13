using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Models;
using Microsoft.Extensions.Logging;
using Moq;

namespace DeadReckoningAdapter.Tests.Core.Services
{
    public class LocationCalculatorServiceTests
    {
        private readonly Mock<ILogger<LocationCalculatorService>> _mockLogger;

        public LocationCalculatorServiceTests()
        {
            _mockLogger = new Mock<ILogger<LocationCalculatorService>>();
        }

        [Fact]
        public void FindVP_ShouldReturnNewLocation()
        {
            // Arrange
            var deadReckonMessage = new DeadReckoningMessage
            {
                Id = "456",
                Position = new PositionDR
                {
                    Odometer = 100
                },
                Trip = new Trip
                {
                    TripId = "123",
                    RouteId = "routeId"
                },
                Vehicle = new VehicleDR
                {
                    Id = "456"
                },
                ReceivedAt = 123,
            };
            var cachedVP = new VehiclePositionMessage
            {
                Id = "456",
                VehiclePosition = new VehiclePosition
                {
                    Position = new Position
                    {
                        Odometer = 50,
                        Longitude = 174.66678,
                        Latitude = -36.39801
                    },
                    Trip = new Trip
                    {
                        TripId = "123",
                        RouteId = "555",
                        DirectionId = 0,
                    },
                    Vehicle = new Vehicle
                    {
                        Id = "456"
                    },
                    Timestamp = "123",
                }
            };
            var routeShape = new List<PathPoint> {
                new PathPoint {
                    Longitude = 174.66678,
                    Latitude = -36.39801
                },
                new PathPoint {
                    Longitude = 174.66675,
                    Latitude = -36.39806
                },
                new PathPoint {
                    Longitude = 174.66685,
                    Latitude = -36.3981
                },
                new PathPoint {
                    Longitude = 174.66629,
                    Latitude = -36.39894
                },
                new PathPoint {
                    Longitude = 174.66605,
                    Latitude = -36.39927
                }
            };
            var locationCalculatorService = new LocationCalculatorService(_mockLogger.Object);

            // Act
            var result = locationCalculatorService.FindVP(deadReckonMessage, cachedVP, routeShape);

            // Assert
            Assert.NotNull(result);
            Assert.Equal(174.66660926722338, result.Longitude);
            Assert.Equal(-36.39846109916495, result.Latitude);
        }

        [Fact]
        public void FindVP_ShouldReturnNull_WhenCachedVPIsMissingOdometer()
        {
            // Arrange
            var deadReckonMessage = new DeadReckoningMessage
            {
                Id = "456",
                Position = new PositionDR
                {
                    Odometer = 100
                },
                Trip = new Trip
                {
                    TripId = "123",
                    RouteId = "routeId"
                },
                Vehicle = new VehicleDR
                {
                    Id = "456"
                },
                ReceivedAt = 123,
            };
            var cachedVP = new VehiclePositionMessage
            {
                Id = "456",
                VehiclePosition = new VehiclePosition
                {
                    Position = new Position
                    {
                        Longitude = 174.66678,
                        Latitude = -36.39801
                    },
                    Trip = new Trip
                    {
                        TripId = "123",
                        RouteId = "555",
                        DirectionId = 0,
                    },
                    Vehicle = new Vehicle
                    {
                        Id = "456"
                    },
                    Timestamp = "123",
                }
            };
            var routeShape = new List<PathPoint> {
                new PathPoint {
                    Longitude = 174.66678,
                    Latitude = -36.39801
                },
                new PathPoint {
                    Longitude = 174.66675,
                    Latitude = -36.39806
                },
                new PathPoint {
                    Longitude = 174.66685,
                    Latitude = -36.3981
                },
                new PathPoint {
                    Longitude = 174.66629,
                    Latitude = -36.39894
                },
                new PathPoint {
                    Longitude = 174.66605,
                    Latitude = -36.39927
                }
            };

            var locationCalculatorService = new LocationCalculatorService(_mockLogger.Object);

            // Act
            var result = locationCalculatorService.FindVP(deadReckonMessage, cachedVP, routeShape);

            // Assert
            Assert.Null(result);
        }
    }
}
