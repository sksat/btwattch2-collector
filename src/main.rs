use std::convert::TryInto;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::error::Error;
use std::time::Duration;
use uuid::Uuid;

//const LIGHT_CHARACTERISTIC_UUID: Uuid = uuid_from_str("6e400003-b5a3-f393-e0a9-e50e24dcca9e");
use tokio::time;

use futures::stream::StreamExt;

async fn is_btwattch2(peripheral: &Peripheral) -> bool {
    if peripheral
        .properties()
        .await
        .unwrap()
        .unwrap()
        .local_name
        .iter()
        .any(|name| name.contains("BTWATTCH2"))
    {
        return true;
    }
    false
}

async fn find_btwattch(central: &Adapter) -> Vec<Peripheral> {
    let peripherals = central.peripherals().await.unwrap();
    futures::stream::iter(peripherals)
        .filter_map(|p| async {
            if is_btwattch2(&p).await {
                Some(p)
            } else {
                None
            }
        })
        .collect()
        .await
    //let tmp = join_all(it).await;
    //tmp.iter().collect::<Vec<Peripheral>>()
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

    let btwattch = find_btwattch(&central).await;
    println!("btwattch: {:?}", btwattch);

    // connect to the device
    for bw in btwattch.iter() {
        bw.connect().await?;
        if bw.is_connected().await? {
            println!(
                "connected: {}",
                bw.properties().await?.unwrap().local_name.unwrap()
            );
        }
        bw.discover_services().await?;

        // find the characteristic we want
        let chars = bw.characteristics();
        let rx_uuid = Uuid::parse_str("6e400003-b5a3-f393-e0a9-e50e24dcca9e").unwrap();
        let tlm_char = chars
            .iter()
            .find(|c| {
                println!("{}", c.uuid);
                c.uuid == rx_uuid
            })
            .expect("Unable to find characterics");
        bw.subscribe(tlm_char).await?
    }

    let chars = btwattch[0].characteristics();
    let mut chars_it = chars.iter();
    let tx_uuid = Uuid::parse_str("6e400002-b5a3-f393-e0a9-e50e24dcca9e").unwrap();
    let cmd_char = chars_it
        .find(|c| {
            println!("{}", c.uuid);
            c.uuid == tx_uuid
        })
        .expect("Unable to find characterics");

    let btw_nstream: Vec<_> = futures::stream::iter(btwattch.clone())
        .then(|bw| async move { (bw.address(), bw.notifications().await.unwrap()) })
        //.map(|bw| (true, false))
        .collect()
        .await;

    let cmd = cmd_char.clone();
    tokio::spawn(async move {
        let _rt = tokio::runtime::Runtime::new().unwrap();
        loop {
            let _payload = vec![0xAA, 0x00, 0x01, 0x83];
            let payload = vec![0xAA, 0x00, 0x01, 0x08, 0xB3];

            //println!("send");

            for bw in btwattch.iter() {
                bw.write(&cmd, &payload, WriteType::WithoutResponse)
                    .await
                    .unwrap();
            }

            time::sleep(Duration::from_millis(1000)).await;
        }
    });

    let len = btw_nstream.len();
    let mut btw_nstream: Vec<(_, Vec<u8>)> =
        btw_nstream.into_iter().zip(vec![Vec::new(); len]).collect();

    //let iclient = influxdb::Client::new("http://pi4.sksat.net:8086" "");
    let itoken = std::env::var("INFLUXDB_TOKEN").unwrap();
    let iclient = influxdb2_client::Client::new("http://pi4.sksat.net:8086", itoken);

    loop {
        for nstream in &mut btw_nstream {
            let data_buf = &mut nstream.1;
            let nstream = &mut nstream.0;

            let address = nstream.0;
            let nstream = &mut nstream.1;
            if let Some(data) = nstream.next().await {
                // receive to buf
                if data.value[0] == 0xAA {
                    *data_buf = data.value;
                } else {
                    data_buf.extend(data.value);
                }

                //println!("recv: {:x?}", data_buf);

                if data_buf.len() < 23 {
                    continue;
                }

                // deserialize

                let mut voltage = vec![0; 6];
                voltage.copy_from_slice(&data_buf[5..11]);
                voltage.extend_from_slice(&[0, 0]);
                let voltage: [u8; 8] = voltage.try_into().unwrap();
                let voltage = i64::from_le_bytes(voltage);
                let voltage = voltage as f64 / 16777216.0;

                let mut current = vec![0; 6];
                current.copy_from_slice(&data_buf[11..17]);
                current.extend_from_slice(&[0, 0]);
                let current: [u8; 8] = current.try_into().unwrap();
                let current = i64::from_le_bytes(current);
                let current = current as f64 / 1073741824.0;

                let mut wattage = vec![0; 6];
                wattage.copy_from_slice(&data_buf[17..23]);
                wattage.extend_from_slice(&[0, 0]);
                let wattage: [u8; 8] = wattage.try_into().unwrap();
                let wattage = i64::from_le_bytes(wattage);
                let wattage = wattage as f64 / 16777216.0;

                println!(
                    "addr = {}, V = {}, A = {}, W = {}",
                    address, voltage, current, wattage
                );

                let point = influxdb2_client::models::DataPoint::builder("btwattch2")
                    .tag("address", address.to_string())
                    .field("voltage", voltage)
                    .field("ampere", current)
                    .field("wattage", wattage)
                    .build()?;

                let org = std::env::var("INFLUXDB_ORG").unwrap();
                let bucket = std::env::var("INFLUXDB_BUCKET").unwrap();
                iclient
                    .write(&org, &bucket, futures::stream::iter(vec![point]))
                    .await?;

                //#[derive(InfluxDbWriteable)]
                //struct Wattch2Data {
                //    time: chrono::DateTime<chrono::Utc>,
                //    #[influxdb(tag)]
                //    address: String,
                //    voltage: f32,
                //    ampere: f32,
                //    wattage: f32,
                //}

                //let address = address.to_string();
                //let time = chrono::Utc::now();
                //let data = Wattch2Data {
                //    time,
                //    address,
                //    voltage,
                //    ampere: current,
                //    wattage,
                //};

                //iclient
                //    .query(data.into_query("btwattch2"))
                //    .await
                //    .expect("failed to write query to InfluxDB");
            }
        }
    }
}
