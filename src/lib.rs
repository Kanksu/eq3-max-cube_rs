use std::io::{BufReader, BufRead, BufWriter, Write, Read};
use std::net::{TcpStream, SocketAddr};
use std::time::Duration;
use anyhow::{Result, anyhow};
use log::debug;

mod messages;

use messages::{from_message_m, Devices, Rooms, DeviceConfig, DeviceMode};

#[derive(Debug)]
struct MaxCube {
    stream: TcpStream,
    rooms: Rooms,
    devices: Devices,
}


impl MaxCube {
    fn new(addr: &SocketAddr) -> Result<Self> {
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
                break;
            } else if received.starts_with('M') {
                (cube.rooms, cube.devices) = from_message_m(&received)?;
            }
        }

        Ok(cube)
    }

    fn set_temperature(&mut self, rf_address: u32, temperature: f64) -> Result<()> {
        let cmd = DeviceConfig::new()
            .set_address(rf_address)
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
        // println!("{:?}", cube);
        cube.set_temperature(1763839, 20.0).unwrap();
    }
}
