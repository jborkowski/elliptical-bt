use std::{sync::{Arc, Mutex}, time::Duration, thread};

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use log::*;

use esp_idf_hal::task::executor::{EspExecutor, Local};

use esp32_nimble::{uuid128, BLEDevice, BLEClient, BLEAdvertisedDevice, BLEAddress, BLEAddressType};

static ELLIPTICAL_RAW_MAC: &str = "00:0C:BF:2B:5C:22";

fn main() {

    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Let's Start!");

    // static ELLIPTICAL_MAC: [u8; 6] = [0x00, 0x0C, 0xBF, 0x2B, 0x5C, 0x22];
    // let target_ellipicatl_mac = BLEAddress::new(ELLIPTICAL_MAC, BLEAddressType::Public);
    
    let executor = EspExecutor::<16, Local>::new();

    let _task = executor.spawn_local(async {
        let ble_device = BLEDevice::take();
        let ble_scan = ble_device.get_scan();
        let connect_device: Arc<Mutex<Option<BLEAdvertisedDevice>>> = Arc::new(Mutex::new(None));
  
        let device0 = connect_device.clone();

        info!("Awaiting connection to the ellipical");

        ble_scan
            .active_scan(true)
            .interval(100)
            .window(99)
            .on_result(move |device| {
                if device.addr().to_string().contains(ELLIPTICAL_RAW_MAC) {
                    BLEDevice::take().get_scan().stop().unwrap();
                    *device0.lock().unwrap() = Some(device.clone());
                }
            });
        
        ble_scan.start(10000).await.unwrap();

        let device = &*connect_device.lock().unwrap();

        if let Some(device) = device {
            info!("Advertised Device: {:?}", device);

            let mut client = BLEClient::new();
            client.on_connect(|client| {
                client.update_conn_params(120, 120, 0, 60).unwrap();
            });
            client.connect(device.addr()).await.unwrap();


            let service = client
                .get_service(uuid128!("49535343-fe7d-4ae5-8fa9-9fafd205e455"))
                .await
                .unwrap();



            let comms_app = uuid128!("49535343-8841-43f4-a8d4-ecbe34729bb3");



            let characteristic = service.get_characteristic(comms_app).await.unwrap();

            info!("Got characteristic");
  
            

            info!("Char UUID {}", format!("{}", characteristic.uuid().to_string()));


            characteristic
                .write_value(&EllipticalCommand::GetEquipmentId.to_bytes(), true).await.unwrap();

            characteristic
                .write_value(&EllipticalCommand::GetSerialNumber.to_bytes(), true).await.unwrap();

            characteristic
                .write_value(&EllipticalCommand::GetVersion.to_bytes(), true).await.unwrap();


            characteristic
                .write_value(&(EllipticalCommand::SetSessionData{byte: 0x03}).to_bytes(), true).await.unwrap();


            let bytes = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, 0xff, 0xff, 0xff];
            characteristic
                .write_value(&(EllipticalCommand::SetInfoValue{bytes}).to_bytes(), true).await.unwrap();

             characteristic
                .write_value(&(EllipticalCommand::SetInfoValue{bytes}).to_bytes(), true).await.unwrap();

            characteristic
                .write_value(&(EllipticalCommand::SetInfoValue{bytes}).to_bytes(), true).await.unwrap();

            let session_init = [ 0x02, 0x00,0x08, 0xff, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];

            characteristic
                .write_value(&(EllipticalCommand::SetDisplay{bytes: session_init}).to_bytes(), true).await.unwrap();
                
            
            characteristic
                .write_value(&EllipticalCommand::GetUsageHours.to_bytes(), true).await.unwrap();

            characteristic
                .write_value(&EllipticalCommand::GetStatus.to_bytes(), true).await.unwrap();

            characteristic
                .write_value(&EllipticalCommand::SetFanSpeed.to_bytes(), true).await.unwrap();

            characteristic
                .write_value(&EllipticalCommand::SetHotKey.to_bytes(), true).await.unwrap();
            
            characteristic
                .write_value(&EllipticalCommand::GetCumulativeKm.to_bytes(), true).await.unwrap();

            characteristic
                .write_value(&EllipticalCommand::GetStatus .to_bytes(), true).await.unwrap();
            let session_init_2 = [ 0x01, 0x00,0x00, 0x02, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];

            characteristic
                .write_value(&(EllipticalCommand::SetDisplay{bytes: session_init_2}).to_bytes(), true).await.unwrap();
 
           // characteristic.subscribe_notify(true).await.unwrap();

            characteristic.on_notify(|data|                 info!("Data: {:?}", data));
            
            loop {
              thread::sleep(Duration::from_secs(10));
              characteristic
                  .write_value(&EllipticalCommand::GetStatus .to_bytes(), true).await.unwrap();                
            }



       

//            client.disconnect().unwrap();
        } 
    }).unwrap();

    executor.run(|| true);


}

enum EllipticalCommand {
    GetEquipmentId,
    GetSerialNumber,
    GetVersion,
    SetSessionData {byte: u8},
    SetInfoValue {bytes: [u8; 20]},
    SetDisplay {bytes: [u8; 24]},
    GetUsageHours,
    GetStatus,
    SetFanSpeed,
    SetHotKey,
    GetCumulativeKm,
}


impl EllipticalCommand {
    const HEADER: u8 = 0xF0;
    
    fn to_bytes(&self) -> Vec<u8> {
        fn cmd_no_params(cmd_code: u8) -> Vec<u8> {
            let mut cmd = vec![EllipticalCommand::HEADER, cmd_code];
            cmd.push(EllipticalCommand::checksum(&cmd));
            cmd
        }

        fn cmd_params(cmd_code: u8, params: &[u8]) -> Vec<u8> {
            let mut cmd: Vec<_> =
                vec![EllipticalCommand::HEADER, cmd_code].iter().chain(params.iter()).cloned().collect();
            cmd.push(EllipticalCommand::checksum(&cmd));
            cmd
            
        }
        
        match self {
            EllipticalCommand::GetEquipmentId => cmd_no_params(0xC9),
            EllipticalCommand::GetSerialNumber => cmd_no_params(0xA4),
            EllipticalCommand::GetVersion => cmd_no_params(0xA3),
            EllipticalCommand::SetSessionData{byte}  => cmd_params(0xC4, &[*byte]),
            EllipticalCommand::SetInfoValue{bytes} => cmd_params(0xAD, bytes),
            EllipticalCommand::SetDisplay {bytes} =>  cmd_params(0xCB, bytes), 
            EllipticalCommand::GetUsageHours => cmd_no_params(0xA5),
            EllipticalCommand::GetStatus => cmd_no_params(0xAC),
            EllipticalCommand::SetFanSpeed => cmd_params(0xCA, &[0x00]),
            EllipticalCommand::SetHotKey => cmd_params(0xCA, &[0x01]),
            EllipticalCommand::GetCumulativeKm => cmd_no_params(0xAB),
        }
    }

    fn checksum(data: &[u8]) -> u8 {
        let mut chksum: u8 = 0;

        for &byte in data {
            chksum = chksum.wrapping_add(byte);
        }

        chksum & 0xFF
    }
}
