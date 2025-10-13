using DeadReckoningAdapter.Core.Exceptions;
using DeadReckoningAdapter.Core.Services;
using DeadReckoningAdapter.Models;
using Moq;
using Moq.Protected;
using System.Net;

namespace DeadReckoningAdapter.Tests.Core.Services
{
    public class HttpApiClientTests
    {
        private readonly Mock<HttpMessageHandler> _mockHttpMessageHandler;
        private readonly HttpApiClient _httpApiClient;

        public HttpApiClientTests()
        {
            _mockHttpMessageHandler = new Mock<HttpMessageHandler>();
            var httpClient = new HttpClient(_mockHttpMessageHandler.Object);
            _httpApiClient = new HttpApiClient(httpClient);
        }

        [Fact]
        public async Task GetRouteShapeAsync_WithExistingRouteId_ShouldReturnDeserializedObject()
        {
            // Arrange
            var url = "https://api.example/shapes?route_id=999";
            var responseContent = new StringContent("[{\"shape_id\":\"123-999-test\", \"shape_wkt\":\"LINESTRING(174.66678 -36.39801,174.66675 -36.39806,174.66685 -36.3981,174.66629 -36.39894,174.66605 -36.39927)\"}]");
            _mockHttpMessageHandler
                .Protected()
                .Setup<Task<HttpResponseMessage>>(
                    "SendAsync",
                    ItExpr.Is<HttpRequestMessage>(req =>
                        req.Method == HttpMethod.Get && req.RequestUri != null &&
                        req.RequestUri.AbsoluteUri.Contains("999")),
                    ItExpr.IsAny<CancellationToken>()
                )
                .ReturnsAsync(new HttpResponseMessage
                {
                    StatusCode = HttpStatusCode.OK,
                    Content = responseContent
                });

            // Act
            var result = await _httpApiClient.GetResultAsync<List<RouteShape>>(url);

            // Assert
            Assert.NotNull(result);
            Assert.Single(result);
            Assert.Equal("123-999-test", result.First().ShapeId);
        }

        [Fact]
        public async Task GetTripSummaryAsync_WithErrorResponse_ShouldThrowException()
        {
            // Arrange
            var url = "https://api.example/shapes?route_id=999";
            _mockHttpMessageHandler
                .Protected()
                .Setup<Task<HttpResponseMessage>>(
                    "SendAsync",
                    ItExpr.IsAny<HttpRequestMessage>(),
                    ItExpr.IsAny<CancellationToken>()
                )
                .ReturnsAsync(new HttpResponseMessage
                {
                    StatusCode = HttpStatusCode.NotFound,
                    ReasonPhrase = "Not found"
                });

            // Act & Assert
            var exception = await Assert.ThrowsAsync<NotFoundException>(() => _httpApiClient.GetResultAsync<List<RouteShape>>(url));
            Assert.Equal("Not found", exception.Message);
        }
    }
}
