use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, time::Duration};

/// represents a heating system device, e.g. thermostat, shutter contact...
/// Only thermostat is supported by now.
#[derive(Debug, Default, Deserialize, Serialize)]
pub enum Device {
    #[default]
    NotSupported,
    HeaterThermostat(HeaterThermostat),
}

/// represents a thermostat of heater
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HeaterThermostat {
    /// RF address of the thermostat
    pub rf_address: u32,
    /// Serial number of the thermostat
    pub serial: String,
    /// Name of the thermostat
    pub name: String,
    /// room id (group id), which the thermostat belongs to
    pub room_id: u8,
    /// current valve position, in percent
    pub valve_position: u8,
    /// current temperature set point (which is displayed on thermostat)
    pub temperature_set: f64,
    /// current temperature, which is measured by the thermostat
    /// this value is not always available
    pub temperature_measured: f64,
    /// if the battery level is low
    pub battery_low: bool,
    /// if the thermostat in error state
    pub error: bool,
    /// if the values from thermostat are valid
    pub valid: bool,
}

/// represents a room/group, which is set up by MAX! software
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Room {
    /// room id (group id)
    pub room_id: u8,
    /// room name
    pub name: String,
    /// RF address of the room
    pub rf_address: u32,
}

/// List of devices
pub type Devices = Vec<Device>;
/// List of rooms
pub type Rooms = Vec<Room>;

/// the function shall not be called directly
pub(super) fn from_message_m(recv: &str) -> Result<(Rooms, Devices)> {
    // assertions
    if !recv.starts_with("M:") {
        return Err(anyhow!(
            "Message `M` expected, but `{}` received.",
            recv.chars().next().unwrap()
        ));
    }

    for (index, part) in recv.split(",").into_iter().enumerate() {
        if index == 0 && part != "M:00" {
            return Err(anyhow!("Chunked M-Message not supported."));
        } else if index == 1 && part != "01" {
            return Err(anyhow!("Chunked M-Message not supported."));
        } else if index == 2 {
            let mut b = VecDeque::from(general_purpose::STANDARD.decode(part)?);
            b.pop_front().ok_or(anyhow!("Unexpected data length."))?;
            b.pop_front().ok_or(anyhow!("Unexpected data length."))?;

            // decode all rooms
            let room_count = b.pop_front().ok_or(anyhow!("Unexpected data length."))? as usize;
            let mut rooms = Rooms::new();
            for _ in vec![0; room_count] {
                let room_id = b.pop_front().ok_or(anyhow!("Unexpected data length."))?;
                let length = b.pop_front().ok_or(anyhow!("Unexpected data length."))? as usize;
                let name =
                    String::from_utf8_lossy(&b.drain(..length).into_iter().collect::<Vec<_>>())
                        .to_string();
                let rf_address = u32::from_be_bytes([
                    0,
                    b.pop_front().ok_or(anyhow!("Unexpected data length."))?,
                    b.pop_front().ok_or(anyhow!("Unexpected data length."))?,
                    b.pop_front().ok_or(anyhow!("Unexpected data length."))?,
                ]);
                let room = Room {
                    room_id,
                    name,
                    rf_address,
                };
                rooms.push(room);
            }

            // decode all devices
            let dev_count = b.pop_front().ok_or(anyhow!("Unexpected data length."))? as usize;
            let mut devices = Devices::new();
            for _ in vec![0; dev_count] {
                let dev_type = b.pop_front().ok_or(anyhow!("Unexpected data length."))?;
                let rf_address = u32::from_be_bytes([
                    0,
                    b.pop_front().ok_or(anyhow!("Unexpected data length."))?,
                    b.pop_front().ok_or(anyhow!("Unexpected data length."))?,
                    b.pop_front().ok_or(anyhow!("Unexpected data length."))?,
                ]);
                let serial =
                    String::from_utf8_lossy(&b.drain(..10).into_iter().collect::<Vec<_>>())
                        .to_string();
                let length = b.pop_front().ok_or(anyhow!("Unexpected data length."))? as usize;
                let name =
                    String::from_utf8_lossy(&b.drain(..length).into_iter().collect::<Vec<_>>())
                        .to_string();
                let room_id = b.pop_front().ok_or(anyhow!("Unexpected data length."))?;
                let device = match dev_type {
                    1 => Device::HeaterThermostat(HeaterThermostat {
                        rf_address,
                        serial,
                        room_id,
                        name,
                        ..Default::default()
                    }),
                    _ => Device::NotSupported,
                };
                devices.push(device);
            }
            return Ok((rooms, devices));
        }
    }

    Err(anyhow!("Message M not well-formatted."))
}

pub(super) fn from_message_l(recv: &str, devices: &mut Devices) -> Result<()> {
    // assertions
    if !recv.starts_with("L:") {
        return Err(anyhow!(
            "Message `L` expected, but `{}` received.",
            recv.chars().next().unwrap()
        ));
    }

    let mut b = VecDeque::from(
        general_purpose::STANDARD.decode(
            recv.split(":")
                .last()
                .ok_or(anyhow!("Message L not well-formatted."))?,
        )?,
    );

    while b.len() > 0 {
        let length = b.pop_front().ok_or(anyhow!("Unexpected data length."))? as usize;
        let mut sub = b.drain(..length).into_iter().collect::<VecDeque<_>>();
        let rf_address = u32::from_be_bytes([
            0,
            sub.pop_front().ok_or(anyhow!("Unexpected data length."))?,
            sub.pop_front().ok_or(anyhow!("Unexpected data length."))?,
            sub.pop_front().ok_or(anyhow!("Unexpected data length."))?,
        ]);
        sub.pop_front().ok_or(anyhow!("Unexpected data length."))?; // unknown field
        let flags = u16::from_be_bytes([
            sub.pop_front().ok_or(anyhow!("Unexpected data length."))?,
            sub.pop_front().ok_or(anyhow!("Unexpected data length."))?,
        ]);

        // get mutable reference from devices
        devices.iter_mut().for_each(|e| {
            if let Device::HeaterThermostat(ts) = e {
                if ts.rf_address == rf_address {
                    ts.battery_low = (flags & 0x80) > 0;
                    ts.error = (flags & 0x800) > 0;
                    ts.valid = (flags & 0x1000) > 0;

                    if length > 6 {
                        ts.valve_position = sub.pop_front().unwrap();
                        ts.temperature_set = sub.pop_front().unwrap() as f64 / 2.0;
                        ts.temperature_measured = u16::from_be_bytes([
                            sub.pop_front()
                                .ok_or(anyhow!("Unexpected data length."))
                                .unwrap(),
                            sub.pop_front()
                                .ok_or(anyhow!("Unexpected data length."))
                                .unwrap(),
                        ]) as f64
                            / 10.0;
                    }
                }
            }
        });
    }

    Ok(())
}

pub(super) fn from_message_n(recv: &str) -> Result<Device> {
        // assertions
        if !recv.starts_with("N:") {
            bail!(
                "Message `N` expected, but `{}` received.",
                recv.chars().next().unwrap()
            );
        }

        let b = Vec::from(
            general_purpose::STANDARD.decode(
                recv.split(":")
                    .last()
                    .ok_or(anyhow!("Message N not well-formatted."))?,
            )?,
        );

        if b.len() < 15 {
            bail!("Message `N` shall have a length of 15 bytes. {} received.", b.len())
        }

        if b[0] != 1 {
            bail!("Device type not supported. Only heater thermostat supported.");
        }

        Ok(Device::HeaterThermostat(
            HeaterThermostat {
                rf_address: u32::from_be_bytes([0, b[1], b[2], b[3]]),
                serial: String::from_utf8_lossy(&b[4..15]).to_string(),
                ..Default::default()
            }
        ))

}

/// Device mode, can be Manual, Auto (other mode, such as Vaccation etc is not supported by now)
#[derive(Debug, Default, Copy, Clone)]
pub enum DeviceMode {
    /// temperature set point is manually set, won't change automatically
    Manual = 1,
    /// temperature set point will be changed automatically according the time scheduling
    #[default]
    Auto = 0,
}

/// DeviceConfig is used to change the device configuration, like temperature set point
#[derive(Default, Debug)]
pub struct DeviceConfig {
    mode: DeviceMode,
    temperature: f64,
    rf_address: u32,
    room_id: u8,
}

impl DeviceConfig {
    /// returns a instant of DeviceConfig with default values
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// set the mode for the set command
    pub fn set_mode(mut self, mode: DeviceMode) -> Self {
        self.mode = mode;
        self
    }

    /// set the temperature set point for the set command
    pub fn set_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    /// set the RF address for the set command
    pub fn set_address(mut self, rf_address: u32) -> Self {
        self.rf_address = rf_address;
        self
    }

    /// set the room id for the set command
    /// If the roomt id is 0, the configuration will be applied on all devices
    pub fn set_room_id(mut self, room_id: u8) -> Self {
        self.room_id = room_id;
        self
    }

    /// build the command payload
    pub fn build(&self) -> String {
        let mut data = vec![0x00u8, 0x04, 0x40, 0x00, 0x00, 0x00];
        data.push((self.rf_address >> 16) as u8);
        data.push((self.rf_address >> 8) as u8);
        data.push(self.rf_address as u8);
        data.push(self.room_id);

        data.push(((self.mode as u8) << 6) | (((self.temperature * 2.0) as u8) & 0x3f));
        let mut cmd = "s:".to_string();
        cmd.push_str(&general_purpose::STANDARD.encode(data));
        cmd.push_str("\r\n");
        cmd
    }
}


/// PairingConfig includes the configuration for pairing a new device
#[derive(Debug)]
pub struct PairingConfig {
    /// Timeout configuration for pairing
    timeout: Duration,
}

impl PairingConfig {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub fn build(&self) -> String {
        // build n-message
        format!("n:{:04x}\r\n", u16::try_from(self.timeout.as_secs()).unwrap_or(0xffff))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_message_m_0() {
        // Test data from: https://github.com/Bouni/max-cube-protocol/blob/master/M-Message.md

        let data =  "M:00,01,VgIEAQNCYWQK7WkCBEJ1cm8K8wADCldvaG56aW1tZXIK8wwEDFNjaGxhZnppbW1lcgr1QAUCCu1pS0VRMDM3ODA0MAZIVCBCYWQBAgrzAEtFUTAzNzk1NDQHSFQgQnVybwICCvMMS0VRMDM3OTU1NhlIVCBXb2huemltbWVyIEJhbGtvbnNlaXRlAwIK83lLRVEwMzc5NjY1GkhUIFdvaG56aW1tZXIgRmVuc3RlcnNlaXRlAwIK9UBLRVEwMzgwMTIwD0hUIFNjaGxhZnppbW1lcgQB";

        let (rooms, _) = from_message_m(&data).unwrap();

        // println!("{:?}, {:?}", rooms, devices);
        assert_eq!(rooms.len(), 4);
        assert_eq!(rooms[0].name, "Bad");
        assert_eq!(rooms[0].rf_address, 716137);
        assert_eq!(rooms[3].name, "Schlafzimmer");
        assert_eq!(rooms[3].rf_address, 718144);
    }

    fn extract_message_m_1() -> (Rooms, Devices) {
        let data = "M:00,01,VgIFAQdCZWRyb29tGuXTAgtMaXZpbmcgcm9vbRrqAQMHS2l0Y2hlbhrnLgQGT2ZmaWNlGun/BQhCYXRocm9vbRrlGAUBGuXTT0VRMjEyMTY0NAdCZWRyb29tAQEa6gFPRVEyMTIyMzU2C0xpdmluZyByb29tAgEa5y5PRVEyMTIxNDc2B0tpdGNoZW4DARrp/09FUTIxMjIzNTMGT2ZmaWNlBAEa5RhPRVEyMTIxNzc0CEJhdGhyb29tBQE=";
        from_message_m(&data).unwrap()
    }

    #[test]
    fn test_message_m_1() {
        let (rooms, devices) = extract_message_m_1();

        // println!("{:?}, {:?}", rooms, devices);
        assert_eq!(rooms.len(), 5);
        assert_eq!(devices.len(), 5);
        match devices.get(4).unwrap() {
            Device::HeaterThermostat(st) => {
                assert_eq!(st.serial, "OEQ2121774");
                assert_eq!(st.rf_address, 1762584);
                assert_eq!(st.name, "Bathroom");
            }
            _ => {
                panic!("Wrong device type!");
            }
        }
    }

    #[test]
    fn test_message_l_1() {
        let data =
            "L:CxrnLgkSGQAmAM0ACxrlGAkSGQAKAAAACxrqAQkSGQApAOMACxrp/wkSGRYnAMoACxrl0wkSmQAoAOAA";
        let (_, mut devices) = extract_message_m_1();
        from_message_l(data, &mut devices).unwrap();
        // println!("{:?}", devices);

        match devices.get(2).unwrap() {
            Device::HeaterThermostat(ts) => {
                assert_eq!(ts.name, "Kitchen");
                assert_eq!(ts.valve_position, 0);
                assert_eq!(ts.temperature_set, 19.0);
            }
            _ => panic!("Wrong device type!"),
        }
    }

    #[test]
    fn test_set_temperature() {
        let (_, d) = extract_message_m_1();
        println!("{:?}", d);

        let s = DeviceConfig::new()
            .set_address(1762771)
            .set_room_id(1)
            .set_mode(DeviceMode::Manual)
            .set_temperature(23.0)
            .build();
        assert_eq!(s, "s:AARAAAAAGuXTAW4=\r\n");
    }
}
