using Newtonsoft.Json;

namespace DeadReckoningAdapter.Models
{
    public class VehiclePositionMessage : BaseMessage
    {
        [JsonProperty("vehicle")]
        public required VehiclePosition VehiclePosition { get; set; }
    }

    public class VehiclePosition
    {
        [JsonProperty("position")]
        public required Position Position { get; set; }
        [JsonProperty("vehicle")]
        public required Vehicle Vehicle { get; set; }
        [JsonProperty("trip")]
        public Trip? Trip { get; set; }
        [JsonProperty("occupancyStatus")]
        public string? OccupancyStatus { get; set; }
        [JsonProperty("timestamp")]
        public required string Timestamp { get; set; }
    }

    public class Position : Location
    {
        [JsonProperty("bearing")]
        public double? Bearing { get; set; }
        [JsonProperty("speed")]
        public double? Speed { get; set; }
        [JsonProperty("odometer")]
        public long? Odometer { get; set; }
    }

    public class Vehicle
    {
        [JsonProperty("id")]
        public required string Id { get; set; }
        [JsonProperty("label")]
        public string? Label { get; set; }
        [JsonProperty("licensePlate")]
        public string? LicensePlate { get; set; }
    }

    public class Trip
    {
        [JsonProperty("tripId")]
        public required string TripId { get; set; }
        [JsonProperty("routeId")]
        public required string RouteId { get; set; }
        [JsonProperty("directionId")]
        public int DirectionId { get; set; }
        [JsonProperty("startTime")]
        public string? StartTime { get; set; }
        [JsonProperty("startDate")]
        public string? StartDate { get; set; }
        [JsonProperty("scheduleRelationship")]
        public string? ScheduleRelationship { get; set; }
    }

    enum ScheduleRelationship
    {
        SCHEDULED = 0,
        ADDED = 1,
        UNSCHEDULED = 2,
        CANCELED = 3
    }
}