using System.Text;
using Jint;

namespace DeadReckoningAdapter.Core.Kafka
{
    // This custom partitioner is required to ensure that the same key always goes to the same partition as other components that use KafkaJS
    // See this link for more information: https://propellerheadnz.atlassian.net/wiki/spaces/TEAMA/pages/3150381062/Kafka+Partitioning+KafkaJS+Issue
    public class ConfluentPartitioner
    {
        private readonly Engine _engine;
        private readonly object _lock = new object();

        public ConfluentPartitioner()
        {
            _engine = new Engine();

            InitialiseEngine();
        }

        private void InitialiseEngine()
        {
            string jsCode = @"
            const SEED = 0x9747b28c;
            const M = 0x5bd1e995;
            const R = 24;
            const toPositive = x => x & 0x7fffffff

            function fetchPartition(data, numPartitions = 12) {
            
            const length = data.length;
            let h = SEED ^ length;
            let length4 = length / 4;

            for (let i = 0; i < length4; i++) {
                const i4 = i * 4;
                let k = 
                (data[i4 + 0] & 0xff) + 
                ((data[i4 + 1] & 0xff) << 8) + 
                ((data[i4 + 2] & 0xff) << 16) + 
                ((data[i4 + 3] & 0xff) << 24);
                k *= M;
                k ^= k >>> R;
                k *= M;
                h *= M;
                h ^= k;
            }

            switch (length % 4) {
                case 3:
                h ^= (data[(length & ~3) + 2] & 0xff) << 16;
                case 2:
                h ^= (data[(length & ~3) + 1] & 0xff) << 8;
                case 1:
                h ^= data[length & ~3] & 0xff;
                h *= M;
            }

            h ^= h >>> 13;
            h *= M;
            h ^= h >>> 15;

            return toPositive(h) % numPartitions;
            }";

            _engine.Execute(jsCode);
        }

        public int FetchPartition(string key)
        {
            lock (_lock)
            {
                var keyBytes = Encoding.UTF8.GetBytes(key);

                var result = _engine.Invoke("fetchPartition", keyBytes);

                return (int)result.AsNumber();
            }
        }
    }
}
