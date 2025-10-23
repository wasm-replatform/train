# Train

Train-related services.

## Quick Start

To run the project locally:

1. Set the environment variables in a `.env` file in the project root (see `.env.example`). 
2. Build the wasm guest (builds `./target/wasm32-wasip2/release/r9k_position.wasm`)
3. Add a service to `compose.yaml` and run with Docker compose:

```bash
cargo build --package train --target wasm32-wasip2 --release
docker compose up
```


## Confluent

kafka broker: lkc-prx9qk-6meoz4.australiaeast.azure.glb.confluent.cloud:9092
kafka username: <your confluent api key>
kafka password: <your confluent api token>
kafka consumer group: <something not preexisting>
kafka topic(s): <topic(s) relevant to your service> (these can be for consumer and producer)

10.24.243.102 or 10.24.243.103 or 10.24.243.104

### /etc/host file

10.24.243.102 lkc-prx9qk-g000-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.103 lkc-prx9qk-g001-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.104 lkc-prx9qk-g002-az1-6meoz4.australiaeast.azure.glb.confluent.cloud

10.24.243.102 lkc-prx9qk-0103-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-0085-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-018b-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-005e-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-00d3-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-0164-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-0066-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-00b9-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-00d3-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-0164-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.244.102 lkc-prx9qk-0181-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.244.103 lkc-prx9qk-0181-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.244.104 lkc-prx9qk-0181-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-0181-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.103 lkc-prx9qk-0181-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.104 lkc-prx9qk-0181-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.104 lkc-prx9qk-0164-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.103 lkc-prx9qk-0164-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-0164-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.244.104 lkc-prx9qk-0164-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.244.103 lkc-prx9qk-0164-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.244.102 lkc-prx9qk-0164-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.104 lkc-prx9qk-00d3-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.103 lkc-prx9qk-00d3-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-00d3-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.244.104 lkc-prx9qk-00d3-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.244.103 lkc-prx9qk-00d3-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.244.102 lkc-prx9qk-00d3-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-0024-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.103 lkc-prx9qk-0024-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.104 lkc-prx9qk-0024-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-0041-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.103 lkc-prx9qk-0041-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.104 lkc-prx9qk-0041-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-0065-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.103 lkc-prx9qk-0065-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.104 lkc-prx9qk-0065-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.102 lkc-prx9qk-00ef-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.103 lkc-prx9qk-00ef-az1-6meoz4.australiaeast.azure.glb.confluent.cloud
10.24.243.104 lkc-prx9qk-00ef-az1-6meoz4.australiaeast.azure.glb.confluent.cloud

