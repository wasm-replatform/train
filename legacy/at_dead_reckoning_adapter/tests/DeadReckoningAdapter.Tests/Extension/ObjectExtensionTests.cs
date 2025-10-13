using DeadReckoningAdapter.Extension;
using DeadReckoningAdapter.Tests.TestModels;

namespace DeadReckoningAdapter.Tests.Extension
{
    public class ObjectExtensionTests
    {
        [Fact]
        public void AllPropertiesNotNullOrEmpty_WhenAllPropertiesAreNonNullAndNonEmpty_ShouldReturnTrue()
        {
            // Arrange
            var testObject = new TestClass
            {
                Property1 = "Value1",
                Property2 = "Value2",
                Property3 = 123,
                ConfluentEnvPrefix = "test-",
                TestObject = new TestClass2()
                {
                    Name = "Test",
                    Value = "Test",
                    GroupPrefix = "test-",
                }
            };

            // Act
            var result = testObject.AllPropertiesNotNullOrEmpty();

            // Assert
            Assert.True(result);
        }

        [Fact]
        public void AllPropertiesNotNullOrEmpty_WhenAnyPropertyIsNull_ShouldReturnFalse()
        {
            // Arrange
            var testObject = new TestClass
            {
                Property1 = null,
                Property2 = "Value2",
                Property3 = 123,
                ConfluentEnvPrefix = "test-",
                TestObject = new TestClass2()
                {
                    Name = "Test",
                    Value = "Test",
                    GroupPrefix = "test-",
                }
            };

            // Act
            var result = testObject.AllPropertiesNotNullOrEmpty();

            // Assert
            Assert.False(result);
        }

        [Fact]
        public void AllPropertiesNotNullOrEmpty_WhenAnyPropertyIsEmptyString_ShouldReturnFalse()
        {
            // Arrange
            var testObject = new TestClass
            {
                Property1 = "",     // Empty string property
                Property2 = "Value2",
                Property3 = 123,
                ConfluentEnvPrefix = "test-",
                TestObject = new TestClass2()
                {
                    Name = "Test",
                    Value = "Test",
                    GroupPrefix = "test-",
                }
            };

            // Act
            var result = testObject.AllPropertiesNotNullOrEmpty();

            // Assert
            Assert.False(result);
        }

        [Fact]
        public void AllPropertiesNotNullOrEmpty_WhenObjectIsNull_ShouldReturnFalse()
        {
            // Arrange
            TestClass? testObject = null;

            // Act
            var result = testObject.AllPropertiesNotNullOrEmpty();

            // Assert
            Assert.False(result);
        }

        [Fact]
        public void AllPropertiesNotNullOrEmpty_WhenValueInNestedObjectIsEmpty_ShouldReturnFalse()
        {
            // Arrange
            var testObject = new TestClass
            {
                Property1 = "Value1",
                Property2 = "Value2",
                Property3 = 123,
                ConfluentEnvPrefix = "test-",
                TestObject = new TestClass2()
                {
                    Name = "Test",
                    Value = "",
                    GroupPrefix = "test-",
                }
            };

            // Act
            var result = testObject.AllPropertiesNotNullOrEmpty();

            // Assert
            Assert.False(result);
        }

        [Fact]
        public void AllPropertiesNotNullOrEmpty_WhenValueOfPrefixIsEmpty_ShouldReturnTrue()
        {
            // Arrange
            var testObject = new TestClass
            {
                Property1 = "Value1",
                Property2 = "Value2",
                Property3 = 123,
                ConfluentEnvPrefix = "",
                TestObject = new TestClass2()
                {
                    Name = "Test",
                    Value = "Test",
                    GroupPrefix = "",
                }
            };

            // Act
            var result = testObject.AllPropertiesNotNullOrEmpty();

            // Assert
            Assert.True(result);
        }
    }
}
