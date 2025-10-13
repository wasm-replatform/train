import { EventPattern, Payload, Ctx } from "@nestjs/microservices";
import { KafkaMessage } from "at-realtime-common/kafka";
import { CommonController } from "at-realtime-common/common";
import { KafkaService } from "../services/kafka-service";
import { SmarTrakEvent } from "at-realtime-common/model";
import { PassengerCountEvent } from "../processors/passenger-count";

@CommonController()
export class KafkaController {
    constructor(private kafkaService: KafkaService) {}

    @EventPattern("*")
    public async handleEvent(@Payload() event: SmarTrakEvent | PassengerCountEvent, @Ctx() message: KafkaMessage) {
        this.kafkaService.process(event, message);
    }
}
