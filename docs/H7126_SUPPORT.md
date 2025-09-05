# H7126 Smart Air Purifier Support

This document describes the comprehensive support added for the Govee H7126 Smart Air Purifier in gove2mqtt.

## Overview

The H7126 is a smart air purifier that supports various operating modes and fan speeds. Based on the [Govee API documentation](https://developer.govee.com), this device uses the `work_mode` capability with specific configurations for different operating modes.

## Device Capabilities

The H7126 supports the following capabilities:

### Power Control
- **Type**: `devices.capabilities.on_off`
- **Instance**: `powerSwitch`
- **Values**: `on` (1), `off` (0)

### Work Mode
- **Type**: `devices.capabilities.work_mode`
- **Instance**: `workMode`
- **Modes**:
  - `Sleeping` (value: 1) - Sleep mode for quiet operation
  - `Low` (value: 2) - Low fan speed
  - `High` (value: 3) - High fan speed
  - `Custom` (value: 4) - Custom mode

### Sensors
- **PM2.5**: Range 0-999 μg/m³
- **Humidity**: Range 0-100%
- **Temperature**: Range -20°C to 60°C
- **Filter Life**: Range 0-100%

### Additional Features
- **Filter Life Reset**: Toggle capability
- **Online Status**: Boolean capability

## Home Assistant Integration

### Entity Types Created

1. **Fan Entity** (`fan.mqtt`)
   - Controls power state
   - Supports preset modes (Sleeping, Low, High, Custom)

2. **Switch Entity** (`switch.mqtt`)
   - Filter life reset toggle

3. **Sensor Entities** (`sensor.mqtt`)
   - PM2.5 sensor
   - Humidity sensor
   - Temperature sensor
   - Filter life sensor

### MQTT Topics

#### Fan Control
- **Preset Command**: `gv2mqtt/fan/{device_id}/set-preset-mode`
- **Preset State**: `gv2mqtt/fan/{device_id}/notify-preset-mode`

#### Power Control
- **Command**: `gv2mqtt/switch/{device_id}/command/powerSwitch`
- **State**: `gv2mqtt/switch/{device_id}/state/powerSwitch`

## Implementation Details

### Preset Mode Implementation

The H7126 uses discrete preset modes rather than a continuous range. The implementation:

1. **Preset Mode Detection**: The device presents modes as discrete options (Sleeping, Low, High, Custom)
2. **No Range Override**: Let the work mode system handle these as preset buttons
3. **Quirk Configuration**: Added comprehensive quirks for H7126 with proper device type and sensor units

### Air Purifier Entity

Created a dedicated `AirPurifier` entity type that:
- Maps to Home Assistant's `fan.mqtt` integration
- Supports preset mode switching (Sleeping, Low, High, Custom)
- Provides proper device classification and icons

### Command Handling

Added MQTT command handlers for:
- `mqtt_fan_preset_mode_command`: Sets work mode by name

## Configuration

### Device Quirks

```rust
Quirk::air_purifier("H7126")
    .with_iot_api_support(true)
    .with_platform_temperature_sensor_units(TemperatureUnits::Celsius)
    .with_platform_humidity_sensor_units(HumidityUnits::RelativePercent)
```

### Work Mode Adjustment

```rust
"H7126" | "H7121" => {
    // H7126/H7121 air purifiers: Show as preset modes instead of slider
    // The device has discrete modes: Sleeping, Low, High, Custom
    // We'll let the work mode system handle this as presets
}
```

## Testing

Comprehensive test coverage includes:
- Work mode parsing and preset mode detection
- Device capability detection
- MQTT command handling
- State synchronization

## Usage in Home Assistant

Once configured, the H7126 will appear in Home Assistant as:

1. **Smart Air Purifier** (fan entity) - Main control interface with power and mode control
2. **Filter Life Reset** (switch entity) - Filter reset toggle
3. **PM2.5**, **Humidity**, **Temperature**, **Filter Life** (sensor entities)

The fan entity provides the most intuitive control interface, allowing users to:
- Turn the device on/off
- Switch between preset modes (Sleeping, Low, High, Custom)
- Monitor air quality and filter status

## Related Models

The H7121 model is also supported with the same configuration, as it shares similar capabilities and the same work mode range issue.

## References

- [Govee Developer Platform](https://developer.govee.com)
- [Govee API Documentation](https://developer.govee.com/reference/control-you-devices)
- [Home Assistant Fan Integration](https://www.home-assistant.io/integrations/fan.mqtt)
