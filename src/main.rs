use std::convert::TryInto;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::error::Error;
use std::time::Duration;
use uuid::Uuid;

//const LIGHT_CHARACTERISTIC_UUID: Uuid = uuid_from_str("6e400003-b5a3-f393-e0a9-e50e24dcca9e");
use tokio::time;

use futures::stream::StreamExt;

async fn find_light(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| {
                println!("name: {}", name);
                name.contains("4D7B")
            })
        {
            let prop = p.properties().await.unwrap().unwrap();
            println!("prop: {:?}", prop);
            return Some(p);
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let central = manager
        .adapters()
        .await
        .expect("Unable to fetch adapter list.")
        .into_iter()
        .next()
        .expect("Unable to find adapters.");

    // start scanning for devices
    central.start_scan(ScanFilter::default()).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    time::sleep(Duration::from_secs(2)).await;

    // find the device we're interested in
    let light = find_light(&central).await.expect("No lights found");

    // connect to the device
    light.connect().await?;

    if light.is_connected().await? {
        println!("connected");
    }

    // discover services and characteristics
    light.discover_services().await?;

    // find the characteristic we want
    let chars = light.characteristics();
    let mut chars_it = chars.iter();
    let tx_uuid = Uuid::parse_str("6e400002-b5a3-f393-e0a9-e50e24dcca9e").unwrap();
    let rx_uuid = Uuid::parse_str("6e400003-b5a3-f393-e0a9-e50e24dcca9e").unwrap();
    let cmd_char = chars_it
        .find(|c| {
            println!("{}", c.uuid);
            c.uuid == tx_uuid
        })
        .expect("Unable to find characterics");
    let tlm_char = chars_it
        .clone()
        .find(|c| {
            println!("{}", c.uuid);
            c.uuid == rx_uuid
        })
        .expect("Unable to find characterics");

    //let data = light.read(&cmd_char).await?;
    //println!("{:?}", data);
    light.subscribe(tlm_char).await?;
    let mut nstream = light.notifications().await?;

    let cmd = cmd_char.clone();
    let l2 = light.clone();
    tokio::spawn(async move {
        let _rt = tokio::runtime::Runtime::new().unwrap();
        loop {
            let _payload = vec![0xAA, 0x00, 0x01, 0x83];
            let payload = vec![0xAA, 0x00, 0x01, 0x08, 0xB3];

            println!("send");
            l2.write(&cmd, &payload, WriteType::WithoutResponse)
                .await
                .unwrap();

            time::sleep(Duration::from_millis(1000)).await;
        }
    });

    let mut data_buf = Vec::new();

    while let Some(data) = nstream.next().await {
        println!("recv: {:x?}", data.value);

        if data.value[0] == 0xAA {
            data_buf = data.value;
            let mut voltage = vec![0; 6];
            voltage.copy_from_slice(&data_buf[5..11]);
            voltage.extend_from_slice(&[0, 0]);
            let voltage: [u8; 8] = voltage.try_into().unwrap();
            let voltage = i64::from_le_bytes(voltage);
            let voltage = voltage as f32 / 16777216.0;
            println!("{}", voltage);
        }
    }
    Ok(())
}
