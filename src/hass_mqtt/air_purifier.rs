use crate::hass_mqtt::base::{Device, EntityConfig, Origin};
use crate::hass_mqtt::instance::{publish_entity_config, EntityInstance};
use crate::hass_mqtt::work_mode::ParsedWorkMode;
use crate::platform_api::DeviceType;
use crate::service::device::Device as ServiceDevice;
use crate::service::hass::{availability_topic, topic_safe_id, HassClient};
use crate::service::state::StateHandle;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::json;

/// <https://www.home-assistant.io/integrations/fan.mqtt>
#[derive(Serialize, Clone, Debug)]
pub struct AirPurifierConfig {
    #[serde(flatten)]
    pub base: EntityConfig,

    pub command_topic: String,
    pub state_topic: String,

    /// Optional percentage slider topics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage_command_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage_state_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_range_min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_range_max: Option<i64>,

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

        // Percentage topics for speed/gearMode control
        let percentage_command_topic = format!(
            "gv2mqtt/fan/{id}/set-percentage",
            id = topic_safe_id(device)
        );
        let percentage_state_topic = format!(
            "gv2mqtt/fan/{id}/notify-percentage",
            id = topic_safe_id(device)
        );

        let unique_id = format!("gv2mqtt-{id}-fan", id = topic_safe_id(device));

        let work_mode = ParsedWorkMode::with_device(device).ok();
        // Only include non-range modes as presets (e.g. Auto/Custom). Skip gearMode
        let preset_modes = work_mode
            .as_ref()
            .map(|wm| {
                wm.modes
                    .values()
                    .filter(|m| m.should_show_as_preset())
                    .map(|m| m.name.to_string())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(|| vec![]);

        // Determine optional speed range from a mode that has a contiguous range
        let mut speed_range_min: Option<i64> = None;
        let mut speed_range_max: Option<i64> = None;
        if let Some(wm) = &work_mode {
            // Prefer gearMode when present
            if let Some(gear) = wm.mode_by_name("gearMode") {
                if let Some(r) = gear.contiguous_value_range() {
                    speed_range_min.replace(r.start);
                    speed_range_max.replace(r.end - 1);
                }
            } else {
                // Or any other contiguous range mode
                for mode in wm.modes.values() {
                    if let Some(r) = mode.contiguous_value_range() {
                        speed_range_min.replace(r.start);
                        speed_range_max.replace(r.end - 1);
                        break;
                    }
                }
            }
        }

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
                percentage_command_topic: speed_range_min.map(|_| percentage_command_topic),
                percentage_state_topic: speed_range_min.map(|_| percentage_state_topic),
                speed_range_min,
                speed_range_max,
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

    async fn notify_state(&self, _client: &HassClient) -> anyhow::Result<()> {
        // Publish current power and preset mode state for the fan entity
        let device = self
            .state
            .device_by_id(&self.device_id)
            .await
            .expect("device to exist");

        if let Some(device_state) = device.device_state() {
            _client
                .publish(
                    &self.air_purifier.state_topic,
                    if device_state.on { "ON" } else { "OFF" },
                )
                .await?;
        }

        // Try to determine the current mode and publish it
        let mut current_mode_num = device.humidifier_work_mode.map(|v| v as i64);
        if current_mode_num.is_none() {
            if let Some(cap) = device.get_state_capability_by_instance("workMode") {
                if let Some(mode_num) = cap.state.pointer("/value/workMode").and_then(|v| v.as_i64()) {
                    current_mode_num.replace(mode_num);
                }
            }
        }

        if let Some(mode_num) = current_mode_num {
            if let Ok(work_modes) = ParsedWorkMode::with_device(&device) {
                if let Some(mode) = work_modes.mode_for_value(&json!(mode_num)) {
                    _client
                        .publish(&self.air_purifier.preset_mode_state_topic, mode.name.to_string())
                        .await?;
                }
            }
        }

        Ok(())
    }

}
