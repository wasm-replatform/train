using DeadReckoningAdapter.Models;
using NetTopologySuite.Geometries;
using NetTopologySuite.LinearReferencing;

namespace DeadReckoningAdapter.Core.Services
{
    public interface ILocationCalculatorService
    {
        Models.Location? FindVP(DeadReckoningMessage deadReckonMessage, VehiclePositionMessage cachedVP, List<PathPoint> routeShape);
    }

    public class LocationCalculatorService : ILocationCalculatorService
    {
        private readonly ILogger<LocationCalculatorService> _logger;

        public LocationCalculatorService(ILogger<LocationCalculatorService> logger) {
            _logger = logger;
        }

        /// <summary>
        /// Find the new VP location for a vehicle based on the odometer difference between the cached VP and the dead reckoning message
        /// using the cached VP location as the starting point on the shape
        /// </summary>
        /// <param name="deadReckonMessage">The Dead Reckoning message with odometer reading</param>
        /// <param name="cachedVP">Last cached VP with GPS locations for Trip Id that matches Dead Reckon Trip Id</param>
        /// <param name="routeShape">The route shape to find the last VP location on and then calculate the distance travelled along this shape based on odometer difference</param>
        /// <returns>New location for vehicle</returns>
        public Models.Location? FindVP(DeadReckoningMessage deadReckonMessage, VehiclePositionMessage cachedVP, List<PathPoint> routeShape)
        {
            if (!cachedVP.VehiclePosition.Position.Odometer.HasValue) {
                _logger.LogError("Cached VP does not have Odometer value");
                return null;
            }

            var lineString = BuildLineString(routeShape);

            // Find the segment start and end points on poly line
            var (segment1, segment2) = FindSegment(lineString, new Coordinate(cachedVP.VehiclePosition.Position.Longitude, cachedVP.VehiclePosition.Position.Latitude));

            if (segment1 == null || segment2 == null)
            {
                return null;
            }

            var closestLocation = FindClosestPointOnSegment(segment1, segment2, new Coordinate(cachedVP.VehiclePosition.Position.Longitude, cachedVP.VehiclePosition.Position.Latitude));

            var newVP = CalculateNewLocationFromPoint(lineString, closestLocation, (double)(deadReckonMessage.Position.Odometer - cachedVP.VehiclePosition.Position.Odometer));

            return new Models.Location { Longitude = newVP.X, Latitude = newVP.Y };
        }

        private Coordinate CalculateNewLocationFromPoint(LineString line, Coordinate startPoint, double distanceInMeters)
        {
            LengthIndexedLine indexedLine = new LengthIndexedLine(line);
            double startIndex = indexedLine.Project(startPoint);

            double accumulatedDistance = 0;
            double currentIndex = startIndex;
            Coordinate previousPoint = indexedLine.ExtractPoint(currentIndex);

            // Traverse along the polyline to match the odometer difference
            while (accumulatedDistance < distanceInMeters && currentIndex < line.Length)
            {
                currentIndex += 0.0001;
                Coordinate nextPoint = indexedLine.ExtractPoint(currentIndex);

                accumulatedDistance += HaversineDistance(previousPoint, nextPoint);
                previousPoint = nextPoint;
            }

            return previousPoint;
        }

        private LineString BuildLineString(List<PathPoint> pathPoints)
        {
            var coordinates = new List<Coordinate>();
            foreach (var point in pathPoints)
            {
                double latitude = point.Latitude;
                double longitude = point.Longitude;
                coordinates.Add(new Coordinate(longitude, latitude));
            }

            LineString shape = new LineString(coordinates.ToArray());

            return shape;
        }

        private (Coordinate?, Coordinate?) FindSegment(LineString polyline, Coordinate coordinate)
        {
            double minDistance = double.MaxValue;
            Coordinate? closestPoint1 = null;
            Coordinate? closestPoint2 = null;

            for (int i = 0; i < polyline.NumPoints - 1; i++)
            {
                Coordinate p1 = polyline.GetCoordinateN(i);
                Coordinate p2 = polyline.GetCoordinateN(i + 1);

                Coordinate closestPoint = FindClosestPointOnSegment(p1, p2, coordinate);

                double distance = HaversineDistance(closestPoint, coordinate);

                if (distance < minDistance)
                {
                    minDistance = distance;
                    closestPoint1 = p1;
                    closestPoint2 = p2;
                }
            }

            return (closestPoint1, closestPoint2);
        }

        private double HaversineDistance(Coordinate c1, Coordinate c2)
        {
            const double EarthRadius = 6378137.0;

            double lat1 = ToRadians(c1.Y);
            double lon1 = ToRadians(c1.X);
            double lat2 = ToRadians(c2.Y);
            double lon2 = ToRadians(c2.X);

            double dLat = lat2 - lat1;
            double dLon = lon2 - lon1;

            double a = Math.Pow(Math.Sin(dLat / 2), 2) +
                       Math.Cos(lat1) * Math.Cos(lat2) * Math.Pow(Math.Sin(dLon / 2), 2);

            double c = 2 * Math.Atan2(Math.Sqrt(a), Math.Sqrt(1 - a));

            return EarthRadius * c;
        }

        private static double ToRadians(double degrees) => degrees * Math.PI / 180;

        private static Coordinate FindClosestPointOnSegment(Coordinate firstPointOnSegment, Coordinate secondPointOnSegment, Coordinate point)
        {
            var lon1 = ToRadians(firstPointOnSegment.X);
            var lat1 = ToRadians(firstPointOnSegment.Y);
            var lon2 = ToRadians(secondPointOnSegment.X);
            var lat2 = ToRadians(secondPointOnSegment.Y);
            var lon3 = ToRadians(point.X);
            var lat3 = ToRadians(point.Y);

            // Calculate vectors for the line segment and the point
            double dLat = lat2 - lat1;
            double dLon = lon2 - lon1;

            // Compute projection scalar t using dot product (in spherical coordinates)
            double t = ((lat3 - lat1) * dLat + (lon3 - lon1) * dLon) /
                       (dLat * dLat + dLon * dLon);

            // Clamp t to the range [0, 1] to ensure the closest point lies on the segment
            t = Math.Clamp(t, 0, 1);

            // Compute the closest point in radians
            double closestLat = lat1 + t * dLat;
            double closestLon = lon1 + t * dLon;

            // Convert back to degrees
            closestLat = closestLat * 180 / Math.PI;
            closestLon = closestLon * 180 / Math.PI;

            return new Coordinate(closestLon, closestLat);
        }
    }
}
