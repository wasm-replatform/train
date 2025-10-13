namespace DeadReckoningAdapter.Models
{
    public class VehiclePositionSettings
    {
        public required int Ttl { get; set; }
        public required string VehiclePositionRedisKey { get; set; }
    }
}
