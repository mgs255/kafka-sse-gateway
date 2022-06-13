# Generic Kafka to SSE gateway

Proof of concept service providing a simple gateway between 
Kafka and SSE ([server-sent-events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events)).  The service relies on rdkafka 
which itself has a runtime dependency on [librdkafka1](https://github.com/edenhill/librdkafka).

The service listens for subscription requests on https and forwards 
messages received on a Kafka topic `SSE.BROADCAST` which contain a header
`sse_topic`.  The value of this topic (a utf-8 string) is the name the 
consumer is expected to use in the subscription request.


