use axum::{
    extract::{Extension, Path, TypedHeader},
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{get, get_service},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use futures::stream::Stream;
use log::{debug, info, warn};
use std::{convert::Infallible, net::SocketAddr, time::Duration};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod kafka;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "kafka_sse_gateway=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let consumer = kafka::KafkaConsumer::new();
    let listener = consumer.clone();

    info!("Spawning kafka consumer thread");
    tokio::spawn(async move {
        listener.start().expect("Unable to start kafka listener");
    });

    let static_files_service = get_service(
        ServeDir::new("assets").append_index_html_on_directories(true),
    )
    .handle_error(|error: std::io::Error| async move {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {}", error),
        )
    });

    // build our application with a route
    let app = Router::new()
        .fallback(static_files_service)
        .route("/sse/subscribe/:id", get(sse_handler))
        .layer(Extension(consumer.clone()))
        .layer(TraceLayer::new_for_http());

    let config = RustlsConfig::from_pem_file(
        std::env::var("SSL_CERTIFICATE_PATH")
            .unwrap_or_else(|_| "self-signed-certs/cert.pem".to_owned()),
        std::env::var("SSL_KEY_PATH").unwrap_or_else(|_| "self-signed-certs/key.pem".to_owned()),
    )
    .await
    .expect("Unable to load certificates");

    let web_port = std::env::var("WEB_PORT").unwrap_or("8080".to_owned()).parse::<u16>().expect("Valid listen port");


    let addr = SocketAddr::from(([0, 0, 0, 0], web_port));
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .expect("Unable to start http server");

    let health = Router::new().route("/health", get(|| async { "ok" }));

    let health_addr = SocketAddr::from(([0,0,0,0], 8081));
    println!("listening on {}", health_addr);
    axum_server::bind(health_addr)
        .serve(health.into_make_service())
        .await
        .unwrap();
}

async fn sse_handler(
    Path(id): Path<String>,
    Extension(consumer): Extension<kafka::KafkaConsumer>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    debug!(
        "In sse_hander - Id: {} {}` connected",
        &id,
        user_agent.as_str()
    );

    let stream = async_stream::stream! {
        let topic = id.as_str();

        loop {
            match consumer.subscribe(topic) {
                Some(mut rx) => {
                    match rx.recv().await {
                        Ok(m) => {
                            debug!("Got a message in topic {}: {:?} - payload: {}", &topic, m, m.payload);
                            yield Ok(Event::default().data(m.payload));
                        },
                        Err(e) => {
                            warn!("Error occurred handling message for topic: {}: {:?}", &topic, e);
                            yield Ok(Event::default().data("error occurred"));
                            break;
                        },
                    }
                },
                None => {
                    warn!("No such listener for topic {topic}");
                    break;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(5))
            .text("keep-alive-text"),
    )
}
