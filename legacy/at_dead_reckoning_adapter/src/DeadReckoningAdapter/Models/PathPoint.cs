using Newtonsoft.Json;

namespace DeadReckoningAdapter.Models
{
    public class PathPoint : Location
    {
        [JsonProperty("z")]
        public int Z { get; set; }

        public PathPoint() { }

        public PathPoint(string latitude, string longitude, int z)
        {
            Latitude = Convert.ToDouble(latitude);
            Longitude = Convert.ToDouble(longitude);
            Z = z;
        }
    }
}
