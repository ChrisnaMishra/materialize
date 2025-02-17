# Copyright Materialize, Inc. and contributors. All rights reserved.
#
# Use of this software is governed by the Business Source License
# included in the LICENSE file at the root of this repository.
#
# As of the Change Date specified in that file, in accordance with
# the Business Source License, use of this software will be governed
# by the Apache License, Version 2.0.

$ set keyschema={
    "type": "record",
    "name": "Key",
    "fields": [
        {"name": "key", "type": "string"}
    ]
  }

$ set schema={
        "type" : "record",
        "name" : "test",
        "fields" : [
            {"name":"f1", "type":"string"},
            {"name":"f2", "type":"long"}
        ]
    }

$ kafka-create-topic topic=upsert partitions=1

$ kafka-ingest format=avro topic=upsert key-format=avro key-schema=${keyschema} schema=${schema}
{"key": "fish"} {"f1": "fish", "f2": 1000}
{"key": "bird1"} {"f1":"goose", "f2": 1}
{"key": "birdmore"} {"f1":"geese", "f2": 2}
{"key": "mammal1"} {"f1": "moose", "f2": 1}
{"key": "bird1"}
{"key": "birdmore"} {"f1":"geese", "f2": 56}
{"key": "mammalmore"} {"f1": "moose", "f2": 42}
{"key": "mammal1"}
{"key": "mammalmore"} {"f1":"moose", "f2": 2}

$ kafka-create-topic topic=metrics-test partitions=1
$ kafka-ingest topic=metrics-test format=bytes
jack,jill
goofus,gallant

> CREATE CONNECTION kafka_conn
  TO KAFKA (BROKER '${testdrive.kafka-addr}', SECURITY PROTOCOL PLAINTEXT);

> CREATE CONNECTION IF NOT EXISTS csr_conn TO CONFLUENT SCHEMA REGISTRY (
    URL '${testdrive.schema-registry-url}'
  );

> CREATE SOURCE metrics_test_source (a, b)
  IN CLUSTER ${arg.single-replica-cluster}
  FROM KAFKA CONNECTION kafka_conn (TOPIC 'testdrive-metrics-test-${testdrive.seed}')
  FORMAT CSV WITH 2 COLUMNS
  INCLUDE OFFSET

> CREATE SOURCE upsert
  IN CLUSTER ${arg.single-replica-cluster}
  FROM KAFKA CONNECTION kafka_conn (TOPIC
  'testdrive-upsert-${testdrive.seed}'
  )
  FORMAT AVRO USING CONFLUENT SCHEMA REGISTRY CONNECTION csr_conn
  INCLUDE OFFSET
  ENVELOPE UPSERT

# Adding a select here so that the ingests after this
# triggers lookup from the upsert state
> SELECT key, f1, f2 FROM upsert
key           f1      f2
------------------------
fish          fish    1000
birdmore      geese   56
mammalmore    moose   2

# statistics are only populated every minute by default
$ set-sql-timeout duration=2minutes

# Also test a source with multiple sub-sources.
> CREATE SOURCE auction_house
  IN CLUSTER ${arg.single-replica-cluster}
  FROM LOAD GENERATOR AUCTION FOR ALL TABLES;

# NOTE: These queries are slow to succeed because the default metrics scraping
# interval is 30 seconds.
> SELECT s.name,
  bool_and(u.snapshot_committed),
  SUM(u.messages_received), SUM(u.updates_staged), SUM(u.updates_committed), SUM(u.bytes_received) > 0, bool_and(u.rehydration_latency IS NOT NULL)
  FROM mz_sources s
  JOIN mz_internal.mz_source_statistics_per_worker u ON s.id = u.id
  WHERE s.name IN ('metrics_test_source')
  GROUP BY s.name
  ORDER BY s.name
metrics_test_source true 2 2 2 true true

> DROP SOURCE metrics_test_source

# Note that only the base-source has `messages_received`, but the sub-sources have `messages_committed`.
# Committed will usually, for auction sources, add up to received, but we don't test this (right now) because of
# jitter on when metrics are produced for each sub-source.
> SELECT s.name,
  bool_and(u.snapshot_committed),
  SUM(u.messages_received) > 0, SUM(u.updates_staged) > 0, SUM(u.updates_committed) > 0, SUM(u.bytes_received) > 0, bool_and(u.rehydration_latency IS NOT NULL)
  FROM mz_sources s
  JOIN mz_internal.mz_source_statistics_per_worker u ON s.id = u.id
  WHERE s.name IN ('accounts', 'auction_house', 'auctions', 'bids', 'organizations', 'users')
  GROUP BY s.name
  ORDER BY s.name
accounts       true      false   true  true  false   true
auction_house  true      true    false false true    true
auctions       true      false   true  true  false   true
bids           true      false   true  true  false   true
organizations  true      false   true  true  false   true
users          true      false   true  true  false   true

> DROP SOURCE auction_house

# Test upsert

# Note that we always count 9 messages received, but can see as low as 3 updates.
# This is because there are 3 active keys, as all the messages could be received in 1 second.
# There could also be up to 11 updates, as updates cause inserts and deletes
# (5 initial inserts, 2 deletes, and 2 updates). In total 3 records should be present in upsert state.
# We can't control this, so have to accept the range.
> SELECT
    s.name,
    bool_and(u.snapshot_committed),
    SUM(u.messages_received),
    SUM(u.updates_staged) BETWEEN 3 AND 11,
    SUM(u.updates_committed) BETWEEN 3 AND 11,
    SUM(u.bytes_received) > 0,
    SUM(u.envelope_state_bytes) > 0,
    SUM(u.envelope_state_records),
    bool_and(u.rehydration_latency IS NOT NULL)
  FROM mz_sources s
  JOIN mz_internal.mz_source_statistics_per_worker u ON s.id = u.id
  WHERE s.name IN ('upsert')
  GROUP BY s.name
  ORDER BY s.name
upsert true 9 true true true true 3 true

# While we can't control how batching works above, we can ensure that this new, later update
# causes 1 more messages to be received, which is 1 update, a delete.
# We use `set-from-sql` to assert this. We will also use this to validate that the
# `envelope_state_bytes` value goes down because of the delete.
$ set-from-sql var=updates-committed
SELECT
    (SUM(u.updates_committed) + 1)::text
  FROM mz_sources s
  JOIN mz_internal.mz_source_statistics_per_worker u ON s.id = u.id
  WHERE s.name IN ('upsert')

$ set-from-sql var=state-bytes
SELECT
    (SUM(u.envelope_state_bytes))::text
  FROM mz_sources s
  JOIN mz_internal.mz_source_statistics u ON s.id = u.id
  WHERE s.name IN ('upsert')

$ kafka-ingest format=avro topic=upsert key-format=avro key-schema=${keyschema} schema=${schema}
{"key": "mammalmore"}

> SELECT s.name,
    bool_and(u.snapshot_committed),
    SUM(u.messages_received),
    SUM(u.updates_staged),
    SUM(u.updates_committed),
    SUM(u.bytes_received) > 0,
    SUM(u.envelope_state_bytes) < ${state-bytes},
    SUM(u.envelope_state_records)
  FROM mz_sources s
  JOIN mz_internal.mz_source_statistics_per_worker u ON s.id = u.id
  WHERE s.name IN ('upsert')
  GROUP BY s.name
  ORDER BY s.name
upsert true 10 "${updates-committed}" "${updates-committed}" true true 2

# check pre-aggregated view
> SELECT s.name,
    u.snapshot_committed,
    u.messages_received,
    u.updates_staged,
    u.updates_committed,
    u.bytes_received > 0,
    u.envelope_state_bytes < ${state-bytes},
    u.envelope_state_records,
    u.rehydration_latency IS NOT NULL
  FROM mz_sources s
  JOIN mz_internal.mz_source_statistics u ON s.id = u.id
  WHERE s.name IN ('upsert')
upsert true 10 "${updates-committed}" "${updates-committed}" true true 2 true

> DROP SOURCE upsert
