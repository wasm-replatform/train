using DeadReckoningAdapter.Core.Kafka;

namespace DeadReckoningAdapter.Tests.Core.Kafka
{
    public class ConfluentPartitionerTests
    {
        [Theory]
        [InlineData("31552", 0)]
        [InlineData("512006897", 1)]
        [InlineData("12154", 2)]
        [InlineData("59823", 3)]
        [InlineData("371422000", 4)]
        [InlineData("12112", 5)]
        [InlineData("599999", 6)]
        [InlineData("31556", 7)]
        [InlineData("512001795", 8)]
        [InlineData("15818", 9)]
        public void FetchPartition_ShouldMatchExpectedValue(string key, int expectedPartition)
        {
            // Arrange
            var partitioner = new ConfluentPartitioner();

            // Act
            var partition = partitioner.FetchPartition(key);

            // Assert
            Assert.Equal(expectedPartition, partition);
        }
    }
}
