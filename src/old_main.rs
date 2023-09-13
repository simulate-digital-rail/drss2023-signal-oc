use rasta_rs::RastaListener;
use sci_rs::scils::{SCILSBrightness, SCILSSignalAspect};
use sci_rs::SCIListener;
use sci_rs::SCIMessageType;
use sci_rs::SCITelegram;
use std::net::SocketAddr;

mod io_config;
mod oc_interface;

fn main() {
    let io_cfg = io_config::get_config(1);

    let addr: SocketAddr = "127.0.0.1:8888".parse().unwrap();
    let listener = RastaListener::try_new(addr, 1337).unwrap();

    let mut receiver = SCIListener::new(listener, "S".to_string());
    let mut luminosity = SCILSBrightness::Night;

    let mut oc = oc_interface::OC {
        main_aspect: Default::default(),
        main_aspect_string: "Off".to_string(),
    };

    receiver
        .listen(|telegram| {
            /*
            println!(
                "Received Telegram: {}",
                telegram.message_type.try_as_scils_message_type().unwrap()
            );
            dbg!(&telegram.sender);
            dbg!(&telegram.receiver);
            dbg!(telegram.payload.used);
            */
            if telegram.message_type == SCIMessageType::scils_show_signal_aspect() {
                println!("Should show signal aspect");
                let status_change =
                    SCILSSignalAspect::try_from(telegram.payload.data.as_slice()).unwrap();
                oc.show_signal_aspect(status_change, io_cfg.clone());
                Some(SCITelegram::scils_signal_aspect_status(
                    &*telegram.receiver,
                    &*telegram.sender,
                    oc.signal_aspect_status(),
                ))
            } else if telegram.message_type == SCIMessageType::scils_change_brightness() {
                let change = SCILSBrightness::try_from(telegram.payload.data[0]).unwrap();
                luminosity = change;
                Some(SCITelegram::scils_brightness_status(
                    &*telegram.receiver,
                    &*telegram.sender,
                    luminosity,
                ))
            } else {
                None
            }
        })
        .unwrap();
}
