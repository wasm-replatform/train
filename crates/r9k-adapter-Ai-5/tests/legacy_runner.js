#!/usr/bin/env node
// Lightweight legacy parity runner.
// Uses ts-node to load legacy conversion logic, provides stub GTFS & BlockMgt services.
// Input: JSON on stdin of shape { trainUpdate: { ... } }
// Output: JSON array of emitted SmarTrak events (including twoTap duplication).

require('ts-node').register({ transpileOnly: true });
const path = require('path');
const moment = require('moment-timezone');

// Load legacy classes
const { R9kToSmartrak } = require(path.resolve(__dirname, '../../../legacy/at_r9k_position_adapter/src/r9k-to-smartrak.ts'));
const { TrainUpdate, ChangeType, MovementType } = require(path.resolve(__dirname, '../../../legacy/at_r9k_position_adapter/src/train-update.ts'));
const { Config } = require(path.resolve(__dirname, '../../../legacy/at_r9k_position_adapter/src/config.ts'));

// Stub GTFS API
class StubGtfsApi { constructor(stops){ this.stops = stops; } getStopInfoByStopCode(code){ return this.stops.find(s=>s.stop_code===code); } }
// Stub Block Mgt API
class StubBlockMgtApi { constructor(labels){ this.labels=labels; } async getVehiclesByExternalRefId(){ return this.labels; } }

(async () => {
  const stdin = await new Promise(res => {
    let data='';
    process.stdin.on('data', c => data += c);
    process.stdin.on('end', () => res(data));
  });
  const input = JSON.parse(stdin);
  const tuData = input.trainUpdate;
  // Rehydrate TrainUpdate minimal properties
  const tu = Object.assign(new TrainUpdate(), tuData);
  // Provide map station -> stop code via Config
  const stops = input.stops || [
    { stop_code: '133', stop_lat: -36.84448, stop_lon: 174.76915 },
    { stop_code: '134', stop_lat: -37.20299, stop_lon: 174.90990 },
    { stop_code: '9218', stop_lat: -36.99412, stop_lon: 174.8770 }
  ];
  const vehicles = input.vehicles || ['EMU 001','EMU 002'];
  const converter = new R9kToSmartrak(new StubGtfsApi(stops), new StubBlockMgtApi(vehicles));
  const baseEvents = await converter.convert(tu);
  // Simulate twoTap duplication (publish twice with +5s increments)
  const FIVE_SEC = 5 * 1000;
  const duplicate = [];
  for (const ev of baseEvents) {
    const first = JSON.parse(JSON.stringify(ev));
    const second = JSON.parse(JSON.stringify(ev));
    second.messageData.timestamp = moment(first.messageData.timestamp).add(FIVE_SEC, 'ms').toDate();
    const third = JSON.parse(JSON.stringify(ev));
    third.messageData.timestamp = moment(first.messageData.timestamp).add(FIVE_SEC*2, 'ms').toDate();
    duplicate.push(second, third); // match Rust two publishes (ignore initial construction event)
  }
  // Normalize each event to plain JSON values
  const norm = e => ({
    eventType: e.eventType,
    receivedAt: moment(e.receivedAt).toISOString(),
    externalId: e.remoteData.externalId,
    latitude: e.locationData.latitude,
    longitude: e.locationData.longitude,
    timestamp: moment(e.messageData.timestamp).toISOString()
  });
  process.stdout.write(JSON.stringify(duplicate.map(norm)));
})();
