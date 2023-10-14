use std::{sync::{Arc, Mutex}, time::Duration, thread};

use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use log::*;

use esp_idf_hal::task::executor::{EspExecutor, Local};

use esp32_nimble::{uuid128, BLEDevice, BLEClient, BLEAdvertisedDevice};

fn main() {
    esp_idf_sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Let's Start!");

    let executor = EspExecutor::<16, Local>::new();

    let _task = executor.spawn_local(async {
        let ble_device = BLEDevice::take();
        let ble_scan = ble_device.get_scan();
        let connect_device: Arc<Mutex<Option<BLEAdvertisedDevice>>> = Arc::new(Mutex::new(None));
  

        let device0 = connect_device.clone();

        
        ble_scan
            .active_scan(true)
            .interval(100)
            .window(99)
            .on_result(move |device| {

                info!("{}", format!("{}", device.name()));
                if device.name().contains("red-dragon") {
                    BLEDevice::take().get_scan().stop().unwrap();
                    *device0.lock().unwrap() = Some(device.clone());

//                    (*device0.lock()) = Some(device.clone());
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
                .get_service(uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa"))
                .await
                .unwrap();

            let uuid = uuid128!("d4e0e0d0-1a2b-11e9-ab14-d663bd873d93");
            let characteristic = service.get_characteristic(uuid).await.unwrap();
            let value = characteristic.read_value().await.unwrap();
            info!(
                "{:?} value: {}",
                uuid,
                core::str::from_utf8(&value).unwrap()
            );

            let uuid = uuid128!("a3c87500-8ed3-4bdf-8a39-a01bebede295");
            let characteristic = service.get_characteristic(uuid).await.unwrap();
            info!("subscribe {:?}", uuid);
            characteristic
                .on_notify(|data| {
                    info!("{}", core::str::from_utf8(data).unwrap());
                })
                .subscribe_notify(false)
                .await
                .unwrap();

            thread::sleep(Duration::from_secs(10));


            client.disconnect().unwrap();
        } 
    }).unwrap();

    executor.run(|| true);


}
