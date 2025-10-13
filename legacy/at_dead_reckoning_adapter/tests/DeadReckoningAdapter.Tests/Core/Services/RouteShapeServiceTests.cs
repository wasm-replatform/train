using DeadReckoningAdapter.Core.Exceptions;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Models;
using Microsoft.Extensions.Logging;
using Moq;

namespace DeadReckoningAdapter.Tests.Core.Services
{
    public class RouteShapeServiceTests
    {
        private readonly Mock<IHttpApiClient> _mockHttpClient;
        private readonly RouteShapeSettings _mockRouteShapeSettings;
        private readonly Mock<ILogger<RouteShapeService>> _mockLogger;
        private readonly Mock<IRedisCacheHelper> _mockRedisCacheHelper;
        private readonly RouteShapeService _routeShapeService;

        public RouteShapeServiceTests()
        {
            _mockHttpClient = new Mock<IHttpApiClient>();
            _mockRouteShapeSettings = new RouteShapeSettings { RouteShapeKey = "at-dead-reckoning-adapter=route-shape", Ttl = 3600, Url = "test-url" };
            _mockLogger = new Mock<ILogger<RouteShapeService>>();
            _mockRedisCacheHelper = new Mock<IRedisCacheHelper>();
            _routeShapeService = new RouteShapeService(_mockHttpClient.Object, _mockRouteShapeSettings, _mockLogger.Object, _mockRedisCacheHelper.Object);
        }

        [Fact]
        public async Task GetRouteShapeByRouteId_RouteShapeFromApi_ShouldReturnsRouteShape()
        {
            // Arrange
            var routeId = "999-111";
            var routeVariant = "555";
            var expectedCacheKey = $"{_mockRouteShapeSettings.RouteShapeKey}:{routeId}:{routeVariant}";
            var routeShape = new List<RouteShape>()
            {
                new RouteShape {
                    ShapeId = "123-555-test",
                    ShapeWKT = "LINESTRING(174.66678 -36.39801,174.66675 -36.39806,174.66685 -36.3981,174.66629 -36.39894,174.66605 -36.39927)"
                }
            };
            _mockRedisCacheHelper.Setup(redis => redis.GetCacheAsync<List<PathPoint>>(It.IsAny<string>())).ReturnsAsync(default(List<PathPoint>));
            _mockHttpClient.Setup(client => client.GetResultAsync<List<RouteShape>>(It.IsAny<string>())).ReturnsAsync(routeShape);

            // Act
            var result = await _routeShapeService.GetRouteShapeByRouteId(routeId, routeVariant);

            // Assert
            _mockRedisCacheHelper.Verify(redis => redis.GetCacheAsync<List<PathPoint>>(It.Is<string>(s => s == expectedCacheKey)), Times.Once);
            Assert.NotNull(result);
            Assert.Equal(5, result.Count);
        }

        [Fact]
        public async Task GetRouteShapeByRouteId_RouteShapeFromRedis_ShouldReturnsRouteShape()
        {
            // Arrange
            var routeId = "999-111";
            var routeVariant = "555";
            var expectedCacheKey = $"{_mockRouteShapeSettings.RouteShapeKey}:{routeId}:{routeVariant}";
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
            _mockRedisCacheHelper.Setup(redis => redis.GetCacheAsync<List<PathPoint>>(It.IsAny<string>())).ReturnsAsync(routeShape);

            // Act
            var result = await _routeShapeService.GetRouteShapeByRouteId(routeId, routeVariant);

            // Assert
            _mockRedisCacheHelper.Verify(redis => redis.GetCacheAsync<List<PathPoint>>(It.Is<string>(s => s == expectedCacheKey)), Times.Once);
            Assert.NotNull(result);
            Assert.Equal(5, result.Count);
        }

        [Fact]
        public async Task GetRouteShapeByRouteId_RouteShapeNotFound_ShouldReturnEmptyList()
        {
            // Arrange
            var routeId = "999-111";
            var routeVariant = "555";
            var expectedCacheKey = $"{_mockRouteShapeSettings.RouteShapeKey}:{routeId}:{routeVariant}";
            _mockRedisCacheHelper.Setup(redis => redis.GetCacheAsync<List<PathPoint>>(It.IsAny<string>())).ReturnsAsync(default(List<PathPoint>));
            _mockHttpClient.Setup(client => client.GetResultAsync<List<RouteShape>>(It.IsAny<string>())).Throws(new NotFoundException("not found"));

            // Act & Assert
            var result = await _routeShapeService.GetRouteShapeByRouteId(routeId, routeVariant);

            // Assert
            Assert.NotNull(result);
            Assert.Empty(result);
        }
    }
}
