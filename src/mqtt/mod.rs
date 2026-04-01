pub mod parser;
pub mod subscriber;

use rumqttc::{AsyncClient, EventLoop, MqttOptions};

use crate::config::MqttConfig;

pub fn create_client(config: &MqttConfig) -> (AsyncClient, EventLoop) {
    let mut options = MqttOptions::new(&config.client_id, &config.host, config.port);
    options.set_keep_alive(std::time::Duration::from_secs(30));
    options.set_max_packet_size(256 * 1024, 256 * 1024); // 256KB max payload

    if !config.username.is_empty() {
        options.set_credentials(&config.username, &config.password);
    }

    AsyncClient::new(options, 100)
}
