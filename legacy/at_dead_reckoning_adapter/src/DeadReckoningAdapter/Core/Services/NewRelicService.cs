namespace DeadReckoningAdapter.Core.Services
{
    public interface INewRelicService
    {
        void IncrementConsumeTopicMetric(string topic);
        void IncrementProduceTopicMetric(string topic);
        void IncrementCustomMetric(string metricName);
    }

    public class NewRelicService : INewRelicService
    {
        private readonly string _appName;

        public NewRelicService()
        {
            _appName = Environment.GetEnvironmentVariable("NEW_RELIC_APP_NAME") ?? "UnknownApp";
        }

        public void IncrementConsumeTopicMetric(string topic)
        {
            this.RecordMetric($"Custom/{_appName}/{topic}/received_message_counter", 1);
        }

        public void IncrementProduceTopicMetric(string topic)
        {
            this.RecordMetric($"Custom/{_appName}/{topic}/published_message_counter", 1);
        }

        public void IncrementCustomMetric(string metricName)
        {
            this.RecordMetric($"Custom/{_appName}/{metricName}", 1);
        }

        protected virtual void RecordMetric(string metricName, int value)
        {
            NewRelic.Api.Agent.NewRelic.RecordMetric(metricName, value);
        }
    }
}
