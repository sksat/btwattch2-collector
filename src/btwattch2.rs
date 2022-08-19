use btleplug::api::{Central, Peripheral as _};
use btleplug::platform::{Adapter, Peripheral};

use futures::stream::StreamExt;

pub async fn is_btwattch2(peripheral: &Peripheral) -> bool {
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

pub async fn find_btwattch(central: &Adapter) -> Vec<Peripheral> {
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
