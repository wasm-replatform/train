using Newtonsoft.Json;

namespace DeadReckoningAdapter.Models
{
    public class DeadReckoningMessage : BaseMessage
    {
        [JsonProperty("receivedAt")]
        public required int ReceivedAt { get; set; }
        [JsonProperty("position")]
        public required PositionDR Position { get; set; }
        [JsonProperty("trip")]
        public required Trip Trip { get; set; }
        [JsonProperty("vehicle")]
        public required VehicleDR Vehicle { get; set; }
    }

    public class PositionDR
    {
        [JsonProperty("odometer")]
        public required long Odometer { get; set; }
    }

    public class VehicleDR
    {
        [JsonProperty("id")]
        public required string Id { get; set; }
    }
}
