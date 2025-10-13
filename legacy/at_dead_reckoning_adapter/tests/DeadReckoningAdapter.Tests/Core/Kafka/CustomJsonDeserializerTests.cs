using Confluent.Kafka;
using Confluent.SchemaRegistry;
using DeadReckoningAdapter.Core.Kafka;
using DeadReckoningAdapter.Models;
using Moq;
using System.Text;
using System.Text.Json;

namespace DeadReckoningAdapter.Tests.Core.Kafka
{
    public class CustomJsonDeserializerTests
    {
        [Fact]
        public void Deserialize_ShouldDeserialiseVehiclePositionMessage()
        {
            // Arrange
            var mockClient = new MockSchemaRegistryClient();
            var schemaId = 111;
            var schemaString = "{\"type\":\"record\",\"name\":\"VehiclePositionMessage\",\"fields\":[{\"name\":\"exampleField\",\"type\":\"string\"}]}";

            var schema = new RegisteredSchema("test", 1, schemaId, schemaString, SchemaType.Json, new List<SchemaReference>());
            mockClient.AddSchema(schemaId, schema);

            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            mockSchemaRegistry
                .Setup(sr => sr.GetSchemaRegistry())
                .Returns(mockClient);

            var deserializer = new CustomJsonDeserializer(mockSchemaRegistry.Object);

            var topic = "realtime-gtfs-vp";
            var context = new SerializationContext(MessageComponentType.Value, topic);
            var sampleData = GenerateSampleVehiclePositionMessage();

            // Act
            var result = deserializer.Deserialize(sampleData, isNull: false, context);

            // Assert
            Assert.NotNull(result);
            Assert.IsType<VehiclePositionMessage>(result);
        }

        [Fact]
        public void Deserialize_ShouldDeserialiseDeadReckoningMessage()
        {
            // Arrange
            var mockClient = new MockSchemaRegistryClient();
            var schemaId = 111;
            var schemaString = "{\"type\":\"record\",\"name\":\"DeadReckoningMessage\",\"fields\":[{\"name\":\"exampleField\",\"type\":\"string\"}]}";

            var schema = new RegisteredSchema("test", 1, schemaId, schemaString, SchemaType.Json, new List<SchemaReference>());
            mockClient.AddSchema(schemaId, schema);

            var mockSchemaRegistry = new Mock<ISchemaRegistry>();
            mockSchemaRegistry
                .Setup(sr => sr.GetSchemaRegistry())
                .Returns(mockClient);

            var deserializer = new CustomJsonDeserializer(mockSchemaRegistry.Object);

            var topic = "realtime-dead-reckoning";
            var context = new SerializationContext(MessageComponentType.Value, topic);
            var sampleData = GenerateSampleDeadReckonMessage();

            // Act
            var result = deserializer.Deserialize(sampleData, isNull: false, context);

            // Assert
            Assert.NotNull(result);
            Assert.IsType<DeadReckoningMessage>(result);
        }

        private byte[] GenerateSampleVehiclePositionMessage()
        {
            var message = new VehiclePositionMessage()
            {
                Id = "testId",
                VehiclePosition = new VehiclePosition()
                {
                    Position = new Position { Bearing = 1, Odometer = 12345, Speed = 60, Latitude = 174.55555, Longitude = 36.3636 },
                    Vehicle = new Vehicle { Id = "vehicleId", Label = "vehicleLabel", LicensePlate = "plateNumber" },
                    Trip = new Trip { TripId = "tripId-999", RouteId = "routeId", DirectionId = 1, StartDate = "11/05/2024", StartTime = "10:00", ScheduleRelationship = "rel" },
                    OccupancyStatus = "FULL",
                    Timestamp = "12345"
                }
            };

            return serialiseMessage(message);
        }

        private byte[] GenerateSampleDeadReckonMessage()
        {
            var message = new DeadReckoningMessage()
            {
                Id = "testId",
                Position = new PositionDR() { Odometer = 55555 },
                Trip = new Trip() { TripId = "tripId-999", RouteId = "routeId" },
                Vehicle = new VehicleDR() { Id = "vehicleId" },
                ReceivedAt = 123456,
            };

            return serialiseMessage(message);
        }

        private byte[] serialiseMessage<T>(T message)
        {
            // serialize the message to a byte array using newtonsoft serializer
            var jsonData = JsonSerializer.Serialize(message);
            byte[] jsonBytes = Encoding.UTF8.GetBytes(jsonData);

            using (var memoryStream = new MemoryStream())
            {
                // Write magic byte (1 byte)
                memoryStream.WriteByte(0x0);

                // Write schema ID (4 bytes, big-endian)
                byte[] schemaIdBytes = BitConverter.GetBytes(111);
                if (BitConverter.IsLittleEndian)
                {
                    Array.Reverse(schemaIdBytes);
                }
                memoryStream.Write(schemaIdBytes, 0, schemaIdBytes.Length);

                // Write the JSON payload
                memoryStream.Write(jsonBytes, 0, jsonBytes.Length);

                // Return the serialized message as a byte array
                return memoryStream.ToArray();
            }
        }
    }

    public class MockSchemaRegistryClient : ISchemaRegistryClient
    {
        private readonly Dictionary<int, Schema> _schemaCache = new();

        public IEnumerable<KeyValuePair<string, string>> Config => throw new NotImplementedException();

        public int MaxCachedSchemas => 1000;

        public void AddSchema(int id, Schema schema)
        {
            _schemaCache[id] = schema;
        }

        public Task<Schema> GetSchemaAsync(int id, CancellationToken cancellationToken = default)
        {
            if (_schemaCache.TryGetValue(id, out var schema))
            {
                return Task.FromResult(schema);
            }

            throw new KeyNotFoundException($"Schema with ID {id} not found.");
        }

        public Task<RegisteredSchema> GetLatestSchemaAsync(string subject)
        {
            foreach (var schema in _schemaCache.Values)
            {
                if (schema.Subject == subject)
                {
                    var registeredSchema = schema as RegisteredSchema;
                    if (registeredSchema == null)
                    {
                        throw new KeyNotFoundException($"Schema with subject {subject} not found.");
                    }
                    return Task.FromResult(registeredSchema);
                }
            }

            throw new KeyNotFoundException($"Schema with subject {subject} not found.");
        }

        public Task<Schema> GetSchemaBySubjectAndIdAsync(string subject, int id, string format)
        {
            if (_schemaCache.TryGetValue(id, out var schema))
            {
                return Task.FromResult(schema);
            }

            throw new KeyNotFoundException($"Schema with ID {id} not found.");
        }

        // Implement other methods as needed
        public Task<int> RegisterSchemaAsync(string subject, string schema, CancellationToken cancellationToken = default)
            => throw new NotImplementedException();
        public Task<int> GetSchemaIdAsync(string subject, string schema, CancellationToken cancellationToken = default)
            => throw new NotImplementedException();
        public Task<IEnumerable<string>> GetAllSubjectsAsync(CancellationToken cancellationToken = default)
            => throw new NotImplementedException();
        public Task<Compatibility> GetCompatibilityAsync(string subject, CancellationToken cancellationToken = default)
            => throw new NotImplementedException();
        public Task<bool> TestCompatibilityAsync(string subject, string schema, CancellationToken cancellationToken = default)
            => throw new NotImplementedException();
        public Task SetCompatibilityAsync(string subject, Compatibility compatibility, CancellationToken cancellationToken = default)
            => throw new NotImplementedException();
        public void Dispose() { }

        public Task<int> RegisterSchemaAsync(string subject, string avroSchema, bool normalize = false)
        {
            throw new NotImplementedException();
        }

        public Task<int> RegisterSchemaAsync(string subject, Schema schema, bool normalize = false)
        {
            throw new NotImplementedException();
        }

        public Task<int> GetSchemaIdAsync(string subject, string avroSchema, bool normalize = false)
        {
            throw new NotImplementedException();
        }

        public Task<int> GetSchemaIdAsync(string subject, Schema schema, bool normalize = false)
        {
            throw new NotImplementedException();
        }

        public Task<Schema> GetSchemaAsync(int id, string format)
        {
            throw new NotImplementedException();
        }

        public Task<RegisteredSchema> LookupSchemaAsync(string subject, Schema schema, bool ignoreDeletedSchemas, bool normalize = false)
        {
            throw new NotImplementedException();
        }

        public Task<RegisteredSchema> GetRegisteredSchemaAsync(string subject, int version, bool ignoreDeletedSchemas = true)
        {
            throw new NotImplementedException();
        }

        public Task<string> GetSchemaAsync(string subject, int version)
        {
            throw new NotImplementedException();
        }

        public Task<RegisteredSchema> GetLatestWithMetadataAsync(string subject, IDictionary<string, string> metadata, bool ignoreDeletedSchemas)
        {
            throw new NotImplementedException();
        }

        public Task<List<string>> GetAllSubjectsAsync()
        {
            throw new NotImplementedException();
        }

        public Task<List<int>> GetSubjectVersionsAsync(string subject)
        {
            throw new NotImplementedException();
        }

        public Task<bool> IsCompatibleAsync(string subject, string avroSchema)
        {
            throw new NotImplementedException();
        }

        public Task<bool> IsCompatibleAsync(string subject, Schema schema)
        {
            throw new NotImplementedException();
        }

        public string ConstructKeySubjectName(string topic, string recordType)
        {
            throw new NotImplementedException();
        }

        public string ConstructValueSubjectName(string topic, string recordType)
        {
            return topic + "-value";
        }

        public Task<Compatibility> GetCompatibilityAsync(string subject)
        {
            throw new NotImplementedException();
        }

        public Task<Compatibility> UpdateCompatibilityAsync(Compatibility compatibility, string subject)
        {
            throw new NotImplementedException();
        }

        public void ClearLatestCaches()
        {
            throw new NotImplementedException();
        }

        public void ClearCaches()
        {
            throw new NotImplementedException();
        }
    }
}
