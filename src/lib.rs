#![crate_name = "eq3_max_cube_rs"]

use async_std::io::BufReader;
use async_std::prelude::*;
use async_std::net::{TcpStream, ToSocketAddrs};
use anyhow::{Result, anyhow};
use log::debug;

pub mod messages;

use messages::{from_message_m, Devices, Rooms, DeviceConfig, DeviceMode, Device};
use serde::Serialize;

use crate::messages::from_message_l;

/// MaxCube represtents a MAX! Cube Gateway.
/// All operations to the devices shall be triggert from hier.
#[derive(Debug, Serialize)]
pub struct MaxCube {
    /// Socket connection to Cube. The connection will be kept alive.
    #[serde(skip_serializing)]
    stream: TcpStream,

    /// A list to all rooms (groups)
    pub rooms: Rooms,

    /// A list to all devices
    pub devices: Devices,
}


impl MaxCube {
    /// starts a connection to MAX! Cube gateway.
    /// The connection will be kept alive.
    /// After successful connection, the cube will send back the meta data and status data of the whole system
    /// immediately. The data will be decoded and stored in this structure.
    /// # Examples
    /// 
    /// ```
    /// use std::net::SocketAddr;
    /// 
    /// let cube = MaxCube::new(&SocketAddr::from(([172, 22, 51, 191], 62910))).await.unwrap();
    /// println!("{:?}", cube);
    /// ```
    pub async fn new<A>(addr: A) -> Result<Self> 
        where A: ToSocketAddrs {
        let stream = TcpStream::connect(addr).await?;

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
            reader.read_line(&mut received).await?;
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

    /// sets the thermostat with the rf_address to the manual mode and the given temperature.
    /// # Examples
    /// 
    /// ```
    /// let mut cube = MaxCube::new(&SocketAddr::from(([172, 22, 51, 191], 62910))).await.unwrap();
    /// cube.set_temperature(1763839, 21.0).await.unwrap();
    /// ```
    pub async fn set_temperature(&mut self, rf_address: u32, temperature: f64) -> Result<()> {

        // the room id must be set, if the room id = 0, all thermostats will be set
        // to the temperature.

        let mut dev_conf = DeviceConfig::new();

        let mut dev_it = self.devices.iter().filter(|e| {
            if let Device::HeaterThermostat(ts) = e {
                ts.rf_address == rf_address
            } else {
                false
            }
        });

        if let Some(dev) = dev_it.next() {
            if let Device::HeaterThermostat(ts) = dev {
                dev_conf = dev_conf.set_room_id(ts.room_id);
            } else {
                return Err(anyhow!("Device type not supported."));
            } 
        } else {
            return Err(anyhow!("Device with RF address {} not found.", rf_address));
        }

        let cmd = dev_conf.set_address(rf_address)
            .set_mode(DeviceMode::Manual)
            .set_temperature(temperature)
            .build();

        self.stream.write_all(cmd.as_bytes()).await?;
        self.stream.flush().await?;

        let mut resp = "".to_string();
        let mut reader = BufReader::new(&self.stream);
        reader.read_line(&mut resp).await?;

        let resp_code = resp.split(',').into_iter().collect::<Vec<_>>().get(1).ok_or(anyhow!("Response not well-formatted."))?.parse::<u8>()?;

        if resp_code == 0 {
            Ok(())
        } else {
            Err(anyhow!("Device configuration failed."))
        }
    }
}
