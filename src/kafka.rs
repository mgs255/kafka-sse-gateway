use log::{debug, error, info, warn};
use rand::distributions::Alphanumeric;
use rand::Rng;
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::{BaseConsumer, Consumer, ConsumerContext};
use rdkafka::message::{BorrowedMessage, Headers, Message};
use rdkafka::{Offset, TopicPartitionList};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::broadcast::{Receiver, Sender};

const KAFKA_TOPIC: &str = "SSE.BROADCAST";
const SSE_TOPIC_HEADER: &str = "sse_topic";
const CLIENT_PREFIX: &str = "sse_broadcast_";
const LOOKBACK: i64 = 1000;

const KAFKA_BOOTSTRAP_SERVER: &str = "KAFKA_BOOTSTRAP_SERVER";
const KAFKA_DEFAULT_SERVER: &str = "localhost:9092";

#[derive(Clone, Debug)]
pub struct SseMessage {
    pub payload: String,
}

impl SseMessage {
    pub fn new(body: String) -> Self {
        Self { payload: body }
    }
}

#[derive(Debug, Clone)]
pub struct KafkaConsumer {
    topics: Arc<Mutex<HashMap<String, Sender<SseMessage>>>>,
}

impl KafkaConsumer {
    pub fn new() -> KafkaConsumer {
        KafkaConsumer {
            topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self) -> Result<i32, Box<dyn Error>> {
        info!("Starting kafka consumer");
        let start_time = rdkafka::util::current_time_millis();
        let group_id = rand_test_group();
        let known_topics = vec![KAFKA_TOPIC];

        info!("Using consumer group id: {}", &group_id);

        let consumer = create_base_consumer(
            &group_id,
            Some(HashMap::from([("auto.offset.reset", "beginning")])),
        );

        info!("Fetching watermarks from kafka for topic: {}", KAFKA_TOPIC);
        let (l, h) = consumer
            .fetch_watermarks(KAFKA_TOPIC, 0, Duration::from_secs(10))
            .unwrap_or((-1, -1));
        debug!("Postitions: {} {}", l, h);

        let offset = std::cmp::min(LOOKBACK, h - l);
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(KAFKA_TOPIC, 0, Offset::OffsetTail(offset))?;

        consumer.assign(&tpl)?;

        consumer
            .subscribe(known_topics.as_slice())
            .map_err(Box::new)?;

        let offsets = consumer.committed(std::time::Duration::from_secs(10));
        info!("Offsets: {:?}", offsets);

        let assignment = consumer.assignment();
        info!("Assignment: {:?}", assignment);

        let subscription = consumer.subscription();
        info!("Subscription: {:?}", subscription);

        loop {
            if let Some(Ok(message)) = consumer.iter().next() {


                if let Some((topic, payload)) = get_sse_topic(&message) {
                    let mut shared_topics = self.topics.lock().unwrap();

                    if shared_topics.contains_key(&topic) {
                        debug!("Returning sender for existing topic: {}", &topic);
                    } else {
                        info!("New topic seen: {}", &topic);
                        let (tx, _) = tokio::sync::broadcast::channel(16);
                        shared_topics.insert(topic.to_string(), tx);
                    };

                    let tx = shared_topics.get(&topic).unwrap();

                    if message.timestamp().to_millis().unwrap() >= start_time {
                        debug!("New message received for topic: {topic}");
                        if tx.receiver_count() > 0 {
                            let sse_message = SseMessage::new(payload);
                            match tx.send(sse_message) {
                                Ok(u) => {
                                    info!("Sent message to {} consumers on topic {}", u, &topic)
                                }
                                Err(e) => {
                                    error!("Unable to send message to topic {} {:?}", &topic, e)
                                }
                            }
                        } else {
                            info!("No subscribers for topic: {}", &topic);
                        }
                    }
                }
            } else {
                sleep(Duration::from_millis(5));
            }
        }
    }

    pub fn subscribe(&self, topic: &str) -> Option<Receiver<SseMessage>> {
        let map = self.topics.lock().unwrap();
        debug!("Topic map size: {}", map.len());
        if let Some(tx) = map.get(topic) {
            let sub = tx.subscribe();
            info!("Receiver count for topic: {topic}: {}", tx.receiver_count());
            return Some(sub);
        }

        warn!("No such topic: {topic}");
        None
    }
}

fn get_sse_topic(message: &BorrowedMessage) -> Option<(String, String)> {
    if let Some(headers) = message.headers() {
        for i in 0..headers.count() {
            if let Some((key, val)) = headers.get(i) {
                if key.eq_ignore_ascii_case(SSE_TOPIC_HEADER) {
                    let value = std::str::from_utf8(val).unwrap();
                    let body = std::str::from_utf8(message.payload().unwrap()).unwrap();
                    debug!("  Header {:#?}: {:?}", key, value);
                    return Some((value.to_owned(), body.to_owned()));
                }
            }
        }
    }

    None
}

fn rand_test_group() -> String {
    let id = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .collect::<Vec<u8>>();
    let rand_id = std::str::from_utf8(id.as_slice()).unwrap();
    format!("{}{}", CLIENT_PREFIX, rand_id)
}

pub struct SseConsumerContext {}
impl ClientContext for SseConsumerContext {}
impl ConsumerContext for SseConsumerContext {}

fn create_base_consumer(
    group_id: &str,
    config_overrides: Option<HashMap<&str, &str>>,
) -> BaseConsumer<SseConsumerContext> {
    consumer_config(group_id, config_overrides)
        .create_with_context(SseConsumerContext {})
        .expect("Something went wrong creating a consumer")
}

pub fn consumer_config(
    group_id: &str,
    config_overrides: Option<HashMap<&str, &str>>,
) -> ClientConfig {
    let mut config = ClientConfig::new();

    let server = std::env::var(KAFKA_BOOTSTRAP_SERVER).unwrap_or_else(|_| {
        warn!("Listening on default broker {KAFKA_DEFAULT_SERVER}");
        KAFKA_DEFAULT_SERVER.to_owned()
    });

    config
        .set("group.id", group_id)
        .set("client.id", format!("{}client", CLIENT_PREFIX))
        .set("bootstrap.servers", server.as_str())
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .set_log_level(RDKafkaLogLevel::Debug);

    if let Some(overrides) = config_overrides {
        for (key, value) in overrides {
            config.set(key, value);
        }
    }

    config
}
