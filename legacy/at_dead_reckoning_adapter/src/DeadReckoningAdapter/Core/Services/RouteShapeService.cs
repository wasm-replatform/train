using DeadReckoningAdapter.Core.Exceptions;
using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Models;

namespace DeadReckoningAdapter.Core.Services
{
    public interface IRouteShapeService
    {
        Task<List<PathPoint>> GetRouteShapeByRouteId(string routeId, string routeVariant);
    }
    public class RouteShapeService : IRouteShapeService
    {
        private readonly IHttpApiClient _httpClient;
        private readonly RouteShapeSettings _routeShapeSettings;
        private readonly ILogger<RouteShapeService> _log;
        private readonly IRedisCacheHelper _redisCache;
        public RouteShapeService(IHttpApiClient httpClient, RouteShapeSettings routeShapeSettings, ILogger<RouteShapeService> log, IRedisCacheHelper redisCache)
        {
            _httpClient = httpClient;
            _routeShapeSettings = routeShapeSettings;
            _log = log;
            _redisCache = redisCache;
        }

        public async Task<List<PathPoint>> GetRouteShapeByRouteId(string routeId, string routeVariant)
        {
            var cacheKey = $"{_routeShapeSettings.RouteShapeKey}:{routeId}:{routeVariant}";
            var result = await _redisCache.GetCacheAsync<List<PathPoint>>(cacheKey);
            if (result is not null)
            {
                return result;
            }
            try
            {
                var response = await _httpClient.GetResultAsync<List<RouteShape>>($"{_routeShapeSettings.Url}/shapes?route_id={routeId}");

                var routeVariantShape = response?.Find(x => x.ShapeId.Split("-")[1] == routeVariant);

                if (routeVariantShape != null)
                {
                    var routeShape = GetRouteFromShapeWKT(routeVariantShape.ShapeWKT);
                    await _redisCache.SetCacheAsync(cacheKey, routeShape, TimeSpan.FromSeconds(_routeShapeSettings.Ttl));
                    return routeShape;
                }
            }
            catch (NotFoundException)
            {
                return new List<PathPoint>();
            }
            catch (Exception ex)
            {
                _log.LogError("Error when producing : {0}", ex.Message);
            }

            return new List<PathPoint>();
        }

        private static List<PathPoint> GetRouteFromShapeWKT(string shapeWKT)
        {
            var coordinates = shapeWKT.Replace("LINESTRING", "").Replace("(", "").Replace(")", "").Split(",");

            var result = new List<PathPoint>();
            if (coordinates.Length <= 0)
            {
                return result;
            }

            foreach (var coordinate in coordinates)
            {
                var splitResult = coordinate.Split(' ').Reverse().ToArray();
                result.Add(new PathPoint(splitResult[0], splitResult[1], 0));
            }
            return result;
        }
    }
}
