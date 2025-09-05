use crate::hass_mqtt::base::{Device, EntityConfig, Origin};
use crate::hass_mqtt::instance::{publish_entity_config, EntityInstance};
use crate::hass_mqtt::work_mode::ParsedWorkMode;
use crate::platform_api::DeviceType;
use crate::service::device::Device as ServiceDevice;
use crate::service::hass::{availability_topic, topic_safe_id, HassClient};
use crate::service::state::StateHandle;
use async_trait::async_trait;
use serde::Serialize;

/// <https://www.home-assistant.io/integrations/fan.mqtt>
#[derive(Serialize, Clone, Debug)]
pub struct AirPurifierConfig {
    #[serde(flatten)]
    pub base: EntityConfig,

    pub command_topic: String,
    pub state_topic: String,

    /// HASS will publish here to change the current mode
    pub preset_mode_command_topic: String,
    /// we will publish the current mode here
    pub preset_mode_state_topic: String,

    /// The list of supported preset modes
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub preset_modes: Vec<String>,

    pub optimistic: bool,
}

impl AirPurifierConfig {
    pub async fn publish(&self, state: &StateHandle, client: &HassClient) -> anyhow::Result<()> {
        publish_entity_config("fan", state, client, &self.base, self).await
    }
}

#[derive(Clone)]
pub struct AirPurifier {
    air_purifier: AirPurifierConfig,
    state: StateHandle,
    device_id: String,
}

impl AirPurifier {
    pub async fn new(device: &ServiceDevice, state: &StateHandle) -> anyhow::Result<Self> {
        let _quirk = device.resolve_quirk();
        let use_iot = device.iot_api_supported() && state.get_iot_client().await.is_some();
        let optimistic = !use_iot;

        // command_topic controls the power state; just route it to
        // the general power switch handler
        let command_topic = format!(
            "gv2mqtt/switch/{id}/command/powerSwitch",
            id = topic_safe_id(device)
        );

        let state_topic = format!("gv2mqtt/fan/{id}/state", id = topic_safe_id(device));

        let preset_mode_command_topic = format!(
            "gv2mqtt/fan/{id}/set-preset-mode",
            id = topic_safe_id(device)
        );
        let preset_mode_state_topic = format!(
            "gv2mqtt/fan/{id}/notify-preset-mode",
            id = topic_safe_id(device)
        );

        let unique_id = format!("gv2mqtt-{id}-fan", id = topic_safe_id(device));

        let work_mode = ParsedWorkMode::with_device(device).ok();
        let preset_modes = work_mode
            .as_ref()
            .map(|wm| wm.get_mode_names())
            .unwrap_or(vec![]);

        Ok(Self {
            air_purifier: AirPurifierConfig {
                base: EntityConfig {
                    availability_topic: availability_topic(),
                    name: if device.device_type() == DeviceType::AirPurifier {
                        None
                    } else {
                        Some("Air Purifier".to_string())
                    },
                    device_class: Some("fan"),
                    origin: Origin::default(),
                    device: Device::for_device(device),
                    unique_id,
                    entity_category: None,
                    icon: Some("mdi:air-purifier".to_string()),
                },
                command_topic,
                state_topic,
                preset_mode_command_topic,
                preset_mode_state_topic,
                preset_modes,
                optimistic,
            },
            device_id: device.id.to_string(),
            state: state.clone(),
        })
    }
}

#[async_trait]
impl EntityInstance for AirPurifier {
    async fn publish_config(&self, state: &StateHandle, client: &HassClient) -> anyhow::Result<()> {
        self.air_purifier.publish(state, client).await
    }

    async fn notify_state(&self, _state: &StateHandle, _client: &HassClient) -> anyhow::Result<()> {
        // State notifications are handled by the individual capability handlers
        Ok(())
    }

}
