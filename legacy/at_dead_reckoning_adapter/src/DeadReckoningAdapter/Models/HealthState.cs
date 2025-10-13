namespace DeadReckoningAdapter.Models
{
    public class HealthState
    {
        public bool ConsumerIsReady { get; set; } = false;
        public bool ProducerIsReady { get; set; } = false;
    }
}
