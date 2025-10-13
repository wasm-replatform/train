using Confluent.Kafka;
using Confluent.Kafka.SyncOverAsync;
using Confluent.SchemaRegistry;
using Confluent.SchemaRegistry.Serdes;
using DeadReckoningAdapter.Models;

namespace DeadReckoningAdapter.Core.Kafka
{
    public class CustomJsonDeserializer : IDeserializer<object>
    {
        private readonly ISchemaRegistry _schemaRegistry;

        public CustomJsonDeserializer(ISchemaRegistry schemaRegistry)
        {
            _schemaRegistry = schemaRegistry;
        }

        public object Deserialize(ReadOnlySpan<byte> data, bool isNull, SerializationContext context)
        {
            var cachedSchemaRegistryClient = _schemaRegistry.GetSchemaRegistry();

            if (isNull) {
                throw new InvalidDataException("Cannot deserialize a null message.");
            }

            if (context.Topic.Contains("realtime-gtfs-vp"))
            {
                var deserializer = new JsonDeserializer<VehiclePositionMessage>(cachedSchemaRegistryClient).AsSyncOverAsync();
                return deserializer.Deserialize(data, isNull, context);
            }
            else if (context.Topic.Contains("realtime-dead-reckoning"))
            {
                var deserializer = new JsonDeserializer<DeadReckoningMessage>(cachedSchemaRegistryClient).AsSyncOverAsync();
                return deserializer.Deserialize(data, isNull, context);
            }
            else
            {
                throw new InvalidOperationException($"No schema defined for topic {context.Topic}");
            }
        }
    }
}
