use uuid::{uuid, Uuid};

use byteorder::{BigEndian, WriteBytesExt};

use btleplug::api::{Central, Peripheral as _};
use btleplug::platform::{Adapter, Peripheral};

use futures::stream::StreamExt;

pub const TX_UUID: Uuid = uuid!("6e400002-b5a3-f393-e0a9-e50e24dcca9e");
pub const RX_UUID: Uuid = uuid!("6e400003-b5a3-f393-e0a9-e50e24dcca9e");

pub const CMD_HEADER: &[u8] = &[0xAA];

pub const CMD_MONITORING: &[u8] = &[0x08];

pub const CRC_8_BTWATTCH2: crc::Algorithm<u8> = crc::Algorithm {
    width: 8,
    poly: 0x85,
    init: 0x00,
    refin: false,
    refout: false,
    xorout: 0x00,
    check: 0x00,
    residue: 0x00,
};

pub fn gen_cmd(payload: &[u8]) -> Vec<u8> {
    let size = {
        let mut wtr = vec![];
        wtr.write_u16::<BigEndian>(payload.len() as u16).unwrap();
        wtr
    };

    let crc8 = crc::Crc::<u8>::new(&CRC_8_BTWATTCH2);
    let crc8 = crc8.checksum(payload);

    let mut p: Vec<u8> = CMD_HEADER.to_vec();
    p.extend(size);
    p.extend(payload);
    p.push(crc8);
    p
}

#[test]
fn test_generate_command() {
    assert_eq!(gen_cmd(CMD_MONITORING), vec![0xAA, 0x00, 0x01, 0x08, 0xB3]);
}

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
