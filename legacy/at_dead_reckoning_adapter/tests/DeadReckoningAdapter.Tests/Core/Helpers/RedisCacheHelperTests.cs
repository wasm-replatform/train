using DeadReckoningAdapter.Core.Helpers;
using DeadReckoningAdapter.Tests.TestModels;
using Moq;
using StackExchange.Redis;
using System.Text.Json;

namespace DeadReckoningAdapter.Tests.Core.Helpers
{
    public class RedisCacheHelperTests
    {
        private readonly Mock<IDatabase> _mockDatabase;
        private readonly RedisCacheHelper _redisCacheHelper;

        public RedisCacheHelperTests()
        {
            _mockDatabase = new Mock<IDatabase>();
            _redisCacheHelper = new RedisCacheHelper(_mockDatabase.Object);
        }

        [Fact]
        public async Task SetCacheAsync_ShouldSerializeValueAndSetCache()
        {
            // Arrange
            string key = "test_key";
            var testValue = new TestObject { Value = "TestValue", Name = "TestName" };
            var jsonValue = JsonSerializer.Serialize(testValue);
            TimeSpan? expiry = TimeSpan.FromMinutes(10);

            _mockDatabase
                .Setup(db => db.StringSetAsync(
                    key,
                    It.Is<RedisValue>(v => v == (RedisValue)jsonValue),
                    expiry,
                    false,
                    When.Always,
                    CommandFlags.None))
                .ReturnsAsync(true);

            // Act
            await _redisCacheHelper.SetCacheAsync(key, testValue, expiry);

            // Assert
            _mockDatabase.Verify(db => db.StringSetAsync(
                key,
                It.Is<RedisValue>(v => v == (RedisValue)jsonValue),
                expiry,
                false,
                When.Always,
                CommandFlags.None), Times.Once);
        }

        [Fact]
        public async Task GetCacheAsync_WhenKeyExists_ShouldReturnDeserializedObject()
        {
            // Arrange
            string key = "test_key";
            var expectedValue = new TestObject { Value = "TestValue", Name = "TestName" };
            var jsonValue = JsonSerializer.Serialize(expectedValue);

            _mockDatabase.Setup(db => db.StringGetAsync(key, It.IsAny<CommandFlags>()))
                         .ReturnsAsync(jsonValue);

            // Act
            var result = await _redisCacheHelper.GetCacheAsync<TestObject>(key);

            // Assert
            Assert.NotNull(result);
            Assert.Equal(expectedValue.Name, result.Name);
            Assert.Equal(expectedValue.Value, result.Value);
        }

        [Fact]
        public async Task GetCacheAsync_WhenKeyDoesNotExist_ShouldReturnDefault()
        {
            // Arrange
            string key = "nonexistent_key";

            _mockDatabase.Setup(db => db.StringGetAsync(key, It.IsAny<CommandFlags>()))
                         .ReturnsAsync(RedisValue.Null);

            // Act
            var result = await _redisCacheHelper.GetCacheAsync<object>(key);

            // Assert
            Assert.Null(result);
        }

        [Fact]
        public async Task RemoveCacheAsync_WhenKeyIsExisting_ShouldDeleteKey()
        {
            // Arrange
            string key = "test_key";

            _mockDatabase.Setup(db => db.KeyDeleteAsync(key, It.IsAny<CommandFlags>()))
                         .ReturnsAsync(true);

            // Act
            await _redisCacheHelper.RemoveCacheAsync(key);

            // Assert
            _mockDatabase.Verify(db => db.KeyDeleteAsync(key, It.IsAny<CommandFlags>()), Times.Once);
        }
    }
}
