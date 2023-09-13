use config_file::FromConfigFile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PinConfig {
    number_of_pins: usize,
    pub(crate) pins_output: Vec<String>,
    pub(crate) pins_output_backup: Vec<String>,
    pub(crate) pins_input: Vec<String>,
    pub(crate) signals: HashMap<String, Vec<u8>>,
}

impl Default for PinConfig {
    fn default() -> Self {
        let off_pins: Vec<u8> = vec![0, 0, 0, 0];
        let ks1_pins: Vec<u8> = vec![1, 1, 1, 1];
        let ks2_pins: Vec<u8> = vec![0, 0, 1, 1];

        Self {
            number_of_pins: 4,
            pins_output: vec![
                "O_1".to_string(),
                "O_2".to_string(),
                "O_3".to_string(),
                "O_4".to_string(),
            ],
            pins_output_backup: vec![
                "O_1".to_string(),
                "O_2".to_string(),
                "O_3".to_string(),
                "O_4".to_string(),
            ],
            pins_input: vec![
                "O_1".to_string(),
                "O_2".to_string(),
                "O_3".to_string(),
                "O_4".to_string(),
            ],
            signals: HashMap::from([
                ("Off".to_string(), off_pins),
                ("Ks1".to_string(), ks1_pins),
                ("Ks2".to_string(), ks2_pins),
            ]),
        }
    }
}

pub fn get_config(config_arg_pos: usize) -> PinConfig {
    let args: Vec<String> = env::args().collect();
    println!("ARGS {:?}", { args.clone() });
    let mut config_file_path = "./config/pin_config.toml";
    let mut cfg = PinConfig::default();
    if args.len() > config_arg_pos {
        config_file_path = &args[config_arg_pos];
    } else {
        println!(
            "NO CONFIG FILE WAS GIVEN, USE DEFAULT PATH {}",
            config_file_path
        )
    }
    println!("CONFIG FILE {:?}", { config_file_path });
    let config_path = Path::new(config_file_path);
    if !config_path.exists() {
        println!("NO CONFIG FILE FOUND, CONTINUE USING DEFAULT VALUES")
    } else {
        cfg = PinConfig::from_config_file(config_file_path).unwrap();
        println!("CONFIG FILE:{:?}", cfg.clone());
        if cfg.pins_output.len() != cfg.number_of_pins && cfg.pins_input.len() != cfg.number_of_pins && cfg.pins_output_backup.len() != cfg.number_of_pins{
            eprintln!("Error: NUMBER OF PINS DOES NOT MATCH, PLEASE CHECK THE CONFIG FILE!");
            std::process::exit(1);
        }
    }
    cfg
}
