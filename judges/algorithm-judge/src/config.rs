use std::sync::LazyLock;

use serde::Deserialize;

fn default_amqp_url() -> String {
    String::from("amqp://127.0.0.1:5672/%2f")
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_amqp_url")]
    pub amqp_url: String,
    pub id: String,
    pub algorithm_queue_name: String,

    pub database_url: String,

    pub s3_bucket: String,

    pub thread_count: usize,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    ::config::Config::builder()
        .add_source(::config::Environment::default().try_parsing(true))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap()
});
