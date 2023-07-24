use std::collections::HashMap;
use std::env;
use rasta_rs::RastaListener;
use sci_rs::scils::{SCILSBrightness, SCILSMain, SCILSSignalAspect};
use sci_rs::SCIListener;
use sci_rs::SCIMessageType;
use sci_rs::SCITelegram;
use std::net::SocketAddr;
use std::path::Path;
use config_file::FromConfigFile;
mod oc_interface;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PinConfig {
    number_of_pins: usize,
    pins:   Vec<String>,
    signals: HashMap<String, Vec<u8>>,
}

impl ::std::default::Default for PinConfig {
    fn default() -> Self {
        let off_pins:Vec<u8> = vec![0,0,0,0];
        let ks1_pins:Vec<u8> = vec![1,1,1,1];
        let ks2_pins:Vec<u8> = vec![0,0,1,1];

        Self {
            number_of_pins: 4,
            pins: vec!["O_1".to_string(), "O_2".to_string(), "O_3".to_string(), "O_4".to_string()],
            signals: HashMap::from([
                ("Off".to_string(), off_pins),
                ("Ks1".to_string(), ks1_pins),
                ("Ks2".to_string(), ks2_pins),
            ])
        }
    }
}

fn main() {
    let cfg = get_config();

    let addr: SocketAddr = "127.0.0.1:8888".parse().unwrap();
    let listener = RastaListener::try_new(addr, 1337).unwrap();

    let mut receiver = SCIListener::new(listener, "S".to_string());
    let mut luminosity = SCILSBrightness::Night;

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
                let status_change = SCILSSignalAspect::try_from(telegram.payload.data.as_slice()).unwrap();
                oc_interface::show_signal_aspect(status_change, cfg.clone());
                Some(SCITelegram::scils_signal_aspect_status(
                    &*telegram.receiver,
                    &*telegram.sender,
                    oc_interface::signal_aspect_status(),
                ))
            }
            else if telegram.message_type == SCIMessageType::scils_change_brightness() {
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

fn get_config() -> PinConfig {
    let args: Vec<String> = env::args().collect();
    println!("ARGS {:?}", { args.clone() });
    let mut config_file_path = "./pin_config.toml";
    let mut cfg = PinConfig::default();
    if args.len() > 1 {
        config_file_path = &args[1];
    } else {
        println!("NO CONFIG FILE WAS GIVEN, USE DEFAULT PATH {}", config_file_path)
    }
    println!("CONFIG FILE {:?}", { config_file_path });
    let config_path = Path::new(config_file_path);
    if !config_path.exists() {
        println!("NO CONFIG FILE PATH WAS GIVEN, CONTINUE USING DEFAULT VALUES")
    } else {
        cfg = PinConfig::from_config_file(config_file_path).unwrap();
        println!("CONFIG FILE:{:?}", cfg.clone());
        if cfg.pins.len() != cfg.number_of_pins {
            eprintln!("Error: NUMBER OF PINS DOES NOR MATCH, PLEASE CHECK THE CONFIG FILE!");
            std::process::exit(1);
        }
    }
    cfg
}