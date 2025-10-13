using System.Text.Json.Serialization;

namespace DeadReckoningAdapter.Models
{
    public class ConfluentSecret
    {
        [JsonPropertyName("key")]
        public required string Key { get; set; }
        [JsonPropertyName("secret")]
        public required string Secret { get; set; }
    }
}
