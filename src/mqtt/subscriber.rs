use chrono::Utc;
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, QoS};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::MqttConfig;
use crate::model::{EnergySample, LoadpointSample, SharedState};
use crate::mqtt::parser;

pub struct SampleBatch {
    pub energy: EnergySample,
    pub loadpoints: Vec<LoadpointSample>,
}

pub async fn run_mqtt_loop(
    client: AsyncClient,
    mut eventloop: EventLoop,
    mqtt_config: MqttConfig,
    current_state: SharedState,
    sample_tx: mpsc::Sender<SampleBatch>,
    sample_interval: Duration,
) {
    let prefix = &mqtt_config.topic_prefix;
    let subscribe_topic = format!("{prefix}/#");
    let mut sample_tick = tokio::time::interval(sample_interval);
    sample_tick.tick().await; // skip first immediate tick

    loop {
        tokio::select! {
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        let topic = &publish.topic;
                        let payload = &publish.payload;

                        if parser::parse_message(prefix, topic, payload) {
                            let mut state = current_state.write().await;
                            parser::apply_message(&mut state, prefix, topic, payload);
                        }
                    }
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        tracing::info!("MQTT connected, re-subscribing to {subscribe_topic}");
                        if let Err(e) = client.subscribe(&subscribe_topic, QoS::AtLeastOnce).await {
                            tracing::error!("Failed to subscribe: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::warn!("MQTT error: {e}. Reconnecting...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    _ => {}
                }
            }
            _ = sample_tick.tick() => {
                let state = current_state.read().await;
                if state.last_updated.is_some() {
                    let now = Utc::now().timestamp();
                    let energy = EnergySample {
                        timestamp: now,
                        grid_power: state.site.grid_power,
                        pv_power: state.site.pv_power,
                        home_power: state.site.home_power,
                        battery_power: state.site.battery_power,
                        battery_soc: state.site.battery_soc,
                    };

                    let loadpoints: Vec<LoadpointSample> = state
                        .loadpoints
                        .iter()
                        .map(|(id, lp)| LoadpointSample {
                            timestamp: now,
                            loadpoint_id: *id,
                            charge_power: lp.charge_power,
                            charged_energy: lp.charged_energy,
                        })
                        .collect();

                    let batch = SampleBatch {
                        energy,
                        loadpoints,
                    };

                    if let Err(e) = sample_tx.send(batch).await {
                        tracing::error!("Failed to send sample to DB writer: {e}");
                    }
                }
            }
        }
    }
}
