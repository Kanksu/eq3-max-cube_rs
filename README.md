# A rust implementation for eQ3 / ELV MAX! Heating system

[![Crates.io Version](https://img.shields.io/crates/v/eq3-max-cube_rs)](https://crates.io/crates/eq3-max-cube_rs)
[![docs.rs](https://img.shields.io/docsrs/eq3-max-cube_rs)](https://docs.rs/eq3-max-cube_rs/latest/)
![Crates.io License](https://img.shields.io/crates/l/eq3-max-cube_rs)
![Crates.io Total Downloads](https://img.shields.io/crates/d/eq3-max-cube_rs)

This crate implements some messages/command to the eQ3 / ELV Max! Cube via TCP connection.

## Usage

```rust
use eq3_max_cube_rs::MaxCube;

#[tokio::main]
async fn main() {
    // connect to Max Cube:
    // `cube` is need to be mutable only if the temperature setting will be changed.
    let mut cube = MaxCube::new(&SocketAddr::from(([172, 22, 51, 191], 62910))).await.unwrap();

    // print the current status of the system
    println!("System: {:?}", cube);

    // set temperature of a thermostat
    cube.set_temperature(1763839, 21.0).await.unwrap();
}

```

Only M-, S-, L-Messsages are implemented. It is enough for operating thermostats though.


## Reference

https://github.com/Bouni/max-cube-protocol/
