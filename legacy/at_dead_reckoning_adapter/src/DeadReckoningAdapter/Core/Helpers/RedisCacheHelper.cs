using StackExchange.Redis;
using System.Text.Json;

namespace DeadReckoningAdapter.Core.Helpers
{
    public interface IRedisCacheHelper
    {
        public Task SetCacheAsync<T>(string key, T value, TimeSpan? expiry = null);
        public Task<T?> GetCacheAsync<T>(string key);
        public Task RemoveCacheAsync(string key);
    }
    public class RedisCacheHelper : IRedisCacheHelper
    {
        private readonly IDatabase _database;
        private readonly JsonSerializerOptions _options;

        public RedisCacheHelper(string connectionString)
        {
            ConnectionMultiplexer _redis = ConnectionMultiplexer.Connect(connectionString);
            _database = _redis.GetDatabase();
            _options = new JsonSerializerOptions
            {
                IncludeFields = true
            };
        }

        public RedisCacheHelper(IDatabase database)
        {
            _database = database;
            _options = new JsonSerializerOptions
            {
                IncludeFields = true
            };
        }

        public async Task SetCacheAsync<T>(string key, T value, TimeSpan? expiry = null)
        {
            string jsonValue = JsonSerializer.Serialize(value);
            await _database.StringSetAsync(key, jsonValue, expiry);
        }

        public async Task<T?> GetCacheAsync<T>(string key)
        {
            var value = await _database.StringGetAsync(key);
            if (!value.HasValue)
                return default;

            return JsonSerializer.Deserialize<T>(value.ToString(), _options);
        }

        public async Task RemoveCacheAsync(string key)
        {
            await _database.KeyDeleteAsync(key);
        }
    }
}
