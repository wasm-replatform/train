using DeadReckoningAdapter.Core.Exceptions;

namespace DeadReckoningAdapter.API.Tests.Core.Exceptions
{
    public class NoConfigurationExceptionTest
    {
        [Fact]
        public void NoConfigurationException_ShouldInheritFromException()
        {
            // Arrange & Act
            var exception = new NoConfigurationException("Test message");

            // Assert
            Assert.IsAssignableFrom<Exception>(exception);
        }

        [Fact]
        public void NoConfigurationException_ShouldSetMessage()
        {
            // Arrange
            var expectedMessage = "Test message";

            // Act
            var exception = new NoConfigurationException(expectedMessage);

            // Assert
            Assert.Equal(expectedMessage, exception.Message);
        }
    }
}