namespace DeadReckoningAdapter.Models
{
    public class KafkaSettings
    {
        public required string BootstrapServers { get; set; }
        public required string ConfluentEnvPrefix { get; set; }
        public required ConsumerSettings ConsumerSettings { get; set; }
        public required ProducerSettings ProducerSettings { get; set; }
    }

    public class ConsumerSettings
    {
        public required string[] Topics { get; set; }
        public required string ConsumerGroup { get; set; }
        public required int BatchSize {get; set;}
        public required string GroupPrefix { get; set; }
    }

    public class ProducerSettings
    {
        public required string Topic { get; set; }
    }
}
