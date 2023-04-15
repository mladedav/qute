use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Cannot deserialize MQTT message: {0:?}")]
    MqttDeserialize(mqttbytes::Error),
}
