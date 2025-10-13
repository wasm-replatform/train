using DeadReckoningAdapter.Core.Exceptions;

namespace DeadReckoningAdapter.Core.Services
{
    public interface IHttpApiClient
    {
        Task<T> GetResultAsync<T>(string url);
    }
    public class HttpApiClient : IHttpApiClient
    {
        private readonly HttpClient _client;
        public HttpApiClient(HttpClient client)
        {
            _client = client;
        }

        public async Task<T> GetResultAsync<T>(string url)
        {

            using (HttpResponseMessage response = await _client.GetAsync(url))
            {
                if (response.IsSuccessStatusCode)
                {
                    var result = await response.Content.ReadFromJsonAsync<T>();
                    if (result is not null)
                        return result;
                }

                throw new NotFoundException(response.ReasonPhrase);
            }
        }
    }
}
