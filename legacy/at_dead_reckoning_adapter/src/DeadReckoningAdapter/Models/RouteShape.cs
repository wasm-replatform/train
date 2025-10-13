using System.Text.Json.Serialization;

namespace DeadReckoningAdapter.Models
{
    public class RouteShape
    {
        [JsonPropertyName("shape_id")]
        public required string ShapeId { get; set; }
        [JsonPropertyName("shape_wkt")]
        public required string ShapeWKT { get; set; }
    }
}
