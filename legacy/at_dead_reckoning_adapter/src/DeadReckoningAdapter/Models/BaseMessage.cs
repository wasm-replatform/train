using Newtonsoft.Json;

namespace DeadReckoningAdapter.Models
{
    public class BaseMessage
    {
        [JsonProperty("id")]
        public required string Id { get; set; }
    }
}
