using DeadReckoningAdapter.Core.Services;

namespace DeadReckoningAdapter.Tests.Core.Services
{
    public class NewRelicServiceTests
    {
        private class TestableNewRelicService : NewRelicService
        {
            public string? RecordedMetricName { get; private set; }
            public int RecordedValue { get; private set; }

            protected override void RecordMetric(string metricName, int value)
            {
                RecordedMetricName = metricName;
                RecordedValue = value;
            }
        }

        [Fact]
        public void IncrementConsumeTopicMetric_ShouldRecordCorrectMetric()
        {
            // Arrange
            var service = new TestableNewRelicService();
            var topic = "TestTopic";
            var expectedMetric = "Custom/UnknownApp/TestTopic/received_message_counter";

            // Act
            service.IncrementConsumeTopicMetric(topic);

            // Assert
            Assert.Equal(expectedMetric, service.RecordedMetricName);
            Assert.Equal(1, service.RecordedValue);
        }

        [Fact]
        public void IncrementProduceTopicMetric_ShouldRecordCorrectMetric()
        {
            // Arrange
            var service = new TestableNewRelicService();
            var topic = "TestTopic";
            var expectedMetric = "Custom/UnknownApp/TestTopic/published_message_counter";

            // Act
            service.IncrementProduceTopicMetric(topic);

            // Assert
            Assert.Equal(expectedMetric, service.RecordedMetricName);
            Assert.Equal(1, service.RecordedValue);
        }

        [Fact]
        public void IncrementCustomMetric_ShouldRecordCorrectMetric()
        {
            // Arrange
            var service = new TestableNewRelicService();
            var metricName = "CustomMetric";
            var expectedMetric = "Custom/UnknownApp/CustomMetric";

            // Act
            service.IncrementCustomMetric(metricName);

            // Assert
            Assert.Equal(expectedMetric, service.RecordedMetricName);
            Assert.Equal(1, service.RecordedValue);
        }

        [Fact]
        public void Constructor_ShouldUseEnvironmentVariableForAppName()
        {
            // Arrange
            var testAppName = "TestApp";
            Environment.SetEnvironmentVariable("NEW_RELIC_APP_NAME", testAppName);
            var service = new TestableNewRelicService();

            // Act
            service.IncrementCustomMetric("TestMetric");

            // Assert
            Assert.Equal($"Custom/{testAppName}/TestMetric", service.RecordedMetricName);
            Assert.Equal(1, service.RecordedValue);

            // Cleanup
            Environment.SetEnvironmentVariable("NEW_RELIC_APP_NAME", null);
        }
    }
}
