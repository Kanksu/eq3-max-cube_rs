use std::io::{BufReader, BufRead, Write};
use std::net::{TcpStream, SocketAddr};
use std::time::Duration;
use anyhow::{Result, anyhow};
use log::debug;

pub mod messages;

use messages::{from_message_m, Devices, Rooms, DeviceConfig, DeviceMode, HeaterThermostat, Device};

use crate::messages::from_message_l;

#[derive(Debug)]
pub struct MaxCube {
    stream: TcpStream,
    pub rooms: Rooms,
    pub devices: Devices,
}


impl MaxCube {
    pub fn new(addr: &SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect_timeout(addr, Duration::from_secs(15))?;

        let mut cube = MaxCube {
            stream,
            rooms: Rooms::new(),
            devices: Devices::new(),
        };

        // Sequence:
        // Receive: H-Message, M-Message (multiple), C-Message (multiple), F-Message, L-Message
        // Only M- and L-Message will be proceed.
        // All the content after L-Message will be ignored.

        // the Max Cube will reply with meta data and status data immediately after connection
        let mut reader = BufReader::new(&cube.stream);

        loop {
            let mut received = String::new();
            reader.read_line(&mut received)?;
            let received = received.replace("\r\n", "");
            debug!("{:?}", received);

            if received.starts_with('L') {
                from_message_l(&received, &mut cube.devices)?;
                break;
            } else if received.starts_with('M') {
                (cube.rooms, cube.devices) = from_message_m(&received)?;
            }
        }

        Ok(cube)
    }

    pub fn set_temperature(&mut self, rf_address: u32, temperature: f64) -> Result<()> {

        // the room id must be set, if the room id = 0, all thermostats will be set
        // to the temperature.

        let mut dev_conf = DeviceConfig::new();

        for dev in self.devices.iter() {
            if let Device::HeaterThermostat(ts) = dev {
                if ts.rf_address == rf_address {
                    dev_conf = dev_conf.set_room_id(ts.room_id);
                    break;
                }
            }
        }


        let cmd = dev_conf.set_address(rf_address)
            .set_mode(DeviceMode::Manual)
            .set_temperature(temperature)
            .build();

        self.stream.write_all(cmd.as_bytes())?;
        self.stream.flush()?;

        let mut resp = "".to_string();
        let mut reader = BufReader::new(&self.stream);
        reader.read_line(&mut resp)?;

        let resp_code = resp.split(',').into_iter().collect::<Vec<_>>().get(1).ok_or(anyhow!("Response not well-formatted."))?.parse::<u8>()?;

        if resp_code == 0 {
            Ok(())
        } else {
            Err(anyhow!("Device configuration failed."))
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_connection() {
        let mut cube = MaxCube::new(&SocketAddr::from(([172, 22, 51, 191], 62910))).unwrap();
        println!("{:?}", cube);
        cube.set_temperature(1763839, 21.0).unwrap();
    }
}
