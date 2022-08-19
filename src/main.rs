use btleplug::api::{BDAddr, Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::Manager;
use std::error::Error;
use std::str::FromStr;
use std::time::Duration;

use tokio::time;

use structopt::StructOpt;

use tracing::{debug, info};

mod btwattch2;

#[derive(Debug, Clone, StructOpt)]
#[structopt(name = "btwattch2-collector")]
struct Opt {
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let opt = Opt::from_args();

    info!("start app");

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

    let btwattch = btwattch2::find_btwattch(&central).await;
    let addr = BDAddr::from_str(&opt.addr).unwrap();
    let bw = btwattch.iter().find(|&bw| bw.address() == addr).unwrap();
    if bw.address() == BDAddr::from_str(&opt.addr).unwrap() {}
    info!("btwattch: {:?}", btwattch);

    // connect to the device
    bw.connect().await?;
    if bw.is_connected().await? {
        info!(
            "connected: {}",
            bw.properties().await?.unwrap().local_name.unwrap()
        );
    }
    bw.discover_services().await?;

    // find the characteristic we want
    let chars = bw.characteristics();
    let tlm_char = chars
        .iter()
        .find(|c| {
            info!("{}", c.uuid);
            c.uuid == btwattch2::RX_UUID
        })
        .expect("Unable to find characterics");
    bw.subscribe(tlm_char).await?;

    let chars = bw.characteristics();
    let mut chars_it = chars.iter();
    let cmd_char = chars_it
        .find(|c| {
            info!("{}", c.uuid);
            c.uuid == btwattch2::TX_UUID
        })
        .expect("Unable to find characterics");

    let cmd = cmd_char.clone();
    let payload = btwattch2::gen_cmd(btwattch2::CMD_TURN_OFF);

    debug!("send");

    bw.write(&cmd, &payload, WriteType::WithoutResponse)
        .await
        .unwrap();

    Ok(())
}
