use crate::hass_mqtt::base::{Device, EntityConfig, Origin};
use crate::hass_mqtt::instance::{publish_entity_config, EntityInstance};
use crate::hass_mqtt::work_mode::ParsedWorkMode;
use crate::platform_api::DeviceType;
use crate::service::device::Device as ServiceDevice;
use crate::service::hass::{
    availability_topic, topic_safe_id, HassClient, fan_in_stabilize_window, fan_pinned_pct,
};
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

    /// Optional percentage slider topics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage_command_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage_state_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage_step: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_range_min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_range_max: Option<i64>,

    /// Optional preset mode topics/modes; omitted for purifier slider-only UX
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_mode_command_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_mode_state_topic: Option<String>,
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
    device_id: String, // raw id for state lookups
    topic_id: String,  // sanitized id used in MQTT topics & stabilizer keys
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

        // We intentionally omit preset mode topics for purifier to keep slider-only UI
        let preset_mode_command_topic: Option<String> = None;
        let preset_mode_state_topic: Option<String> = None;

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

        // No presets; expose percentage-only slider (0..100 with 25% steps)
        let preset_modes: Vec<String> = vec![];
        let speed_range_min: Option<i64> = None;
        let speed_range_max: Option<i64> = None;

        let topic_id = topic_safe_id(device);

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
                percentage_command_topic: Some(percentage_command_topic),
                percentage_state_topic: Some(percentage_state_topic),
                percentage_step: Some(25),
                speed_range_min,
                speed_range_max,
                preset_mode_command_topic,
                preset_mode_state_topic,
                preset_modes,
                optimistic,
            },
            device_id: device.id.to_string(),
            topic_id,
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
        // During stabilization, publish the pinned percentage value keyed by the exact percentage state topic
        let pct_topic_key = self
            .air_purifier
            .percentage_state_topic
            .as_ref()
            .cloned()
            .unwrap_or_default();

        if let Some(pinned) = fan_pinned_pct(&pct_topic_key).await {
            if fan_in_stabilize_window(&pct_topic_key).await {
                _client
                    .publish(&self.air_purifier.state_topic, if pinned > 0 { "ON" } else { "OFF" })
                    .await?;
                let pct_topic = self.air_purifier.percentage_state_topic.as_ref().unwrap();
                _client.publish(pct_topic, pinned.to_string()).await?;
                return Ok(());
            }
        }

        // Publish current power and percentage (mapped 0,25,50,75,100)
        let device = self
            .state
            .device_by_id(&self.device_id)
            .await
            .expect("device to exist");

        if let Some(device_state) = device.device_state() {
            let on = device_state.on;
            _client
                .publish(&self.air_purifier.state_topic, if on { "ON" } else { "OFF" })
                .await?;

            if let Some(pct_topic) = &self.air_purifier.percentage_state_topic {
                // Determine step from (workMode, modeValue)
                let mut step: u8 = 0; // 0=Off
                if on {
                    let mut work_mode_id: Option<i64> = None;
                    let mut mode_value: Option<i64> = None;
                    if let Some(cap) = device.get_state_capability_by_instance("workMode") {
                        work_mode_id = cap
                            .state
                            .pointer("/value/workMode")
                            .and_then(|v| v.as_i64());
                        mode_value = cap
                            .state
                            .pointer("/value/modeValue")
                            .and_then(|v| v.as_i64());
                    }

                    if let (Some(wm_id), Ok(wm)) = (work_mode_id, ParsedWorkMode::with_device(&device)) {
                        let gear_id = wm.mode_by_name("gearMode").and_then(|m| m.value.as_i64());
                        let custom_id = wm
                            .modes
                            .values()
                            .find(|m| m.name.eq_ignore_ascii_case("custom"))
                            .and_then(|m| m.value.as_i64());

                        if Some(wm_id) == gear_id {
                            // use mode_value 1..3
                            let mv = mode_value.unwrap_or(1);
                            step = match mv {
                                1 => 1,
                                2 => 2,
                                3 => 3,
                                _ => 1,
                            };
                        } else if Some(wm_id) == custom_id {
                            step = 4;
                        }
                    }
                }

                let pct = match step {
                    0 => 0,
                    1 => 25,
                    2 => 50,
                    3 => 75,
                    _ => 100,
                };
                _client.publish(pct_topic, pct.to_string()).await?;
            }
        }

        Ok(())
    }

}
