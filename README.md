# A rust implementation for eQ3 / ELV MAX! Heating system

This crate implements some messages/command to the eQ3 / ELV Max! Cube via TCP connection.

## Usage

```rust
use eq3_max_cube_rs::MaxCube;

fn main() {
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
