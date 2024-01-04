use eq3_max_cube_rs::MaxCube;
use std::net::SocketAddr;

#[allow(dead_code)]
async fn list_meta_data() {
    let cube = MaxCube::new("172.22.51.191:62910").await.unwrap();
    println!("{:?}", cube);
}

#[allow(dead_code)]
async fn change_temp() {
    let mut cube = MaxCube::new(&SocketAddr::from(([172, 22, 51, 191], 62910)))
        .await
        .unwrap();
    cube.set_temperature(1763839, 20.0).await.unwrap();
}

#[tokio::main]
async fn main() {
    list_meta_data().await;
    // change_temp().await;
}
