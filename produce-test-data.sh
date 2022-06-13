#!/usr/bin/env bash 

set -x

BROKER=localhost:9092
PAYLOAD='{"test":"testValue"}'
TOPIC='SSE.BROADCAST'
HEADER="sse_topic"
MESSAGE_ID=0

TOPICS="t1 t2 t3 t4 t4 t3 t2 t4 t2 t1 t3 t4 t3 t3"

while true
do
    for t in $TOPICS
    do
        echo $PAYLOAD | kcat -b $BROKER -P -t $TOPIC -H "$HEADER=$t" 
        sleep $(( ( RANDOM % 3 ) ))
    done
done