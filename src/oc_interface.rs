use crate::io_config::PinConfig;
use picontrol::bindings::SPIValue;
use picontrol::PiControl;
use sci_rs::scils::{SCILSMain, SCILSSignalAspect};
pub struct OC {
    pub main_aspect: SCILSMain,
    pub main_aspect_string: String
}



fn show_signal_aspect_internal(oc: &mut OC, signal: &str, cfg: PinConfig) {
    println!("Signal shows {}", signal);
    oc.main_aspect_string = signal.parse().unwrap();
    if cfg.signals.contains_key(signal) {
        let led_values = cfg.signals.get(signal).unwrap();
        let mut pc = PiControl::new().unwrap();
        for (index, value) in led_values.iter().enumerate() {
            let pin = cfg.pins_output.get(index).unwrap();
            set_pin_value(&mut pc, value, &pin);
        }
    } else {
        eprintln!("NO CONFIG FOUND FOR SCI SIGNAL {}", signal)
    }
}



// searches for the given pin and sets the given value
fn set_pin_value(pc: &mut PiControl, value: &u8, pin: &&String) {
    println!("PIN: {}, VALUE: {}", pin, value);

    let var_data = pc.find_variable(&pin);
    let mut val = SPIValue {
        i16uAddress: var_data.i16uAddress,
        i8uBit: var_data.i8uBit,
        i8uValue: *value,
    };
    pc.set_bit_value(&mut val);
}

impl OC {
    pub fn show_signal_aspect(&mut self, signal_aspect: SCILSSignalAspect, cfg: PinConfig) {
        match signal_aspect.main() {
            SCILSMain::Hp0 => show_signal_aspect_internal(self, "Hp0", cfg),
            SCILSMain::Hp0PlusSh1 => show_signal_aspect_internal(self, "Hp0PlusSh1", cfg),
            SCILSMain::Hp0WithDrivingIndicator => {}
            SCILSMain::Ks1 => show_signal_aspect_internal(self, "Ks1", cfg),
            SCILSMain::Ks1Flashing => show_signal_aspect_internal(self, "Ks1Flashing", cfg),
            SCILSMain::Ks1FlashingWithAdditionalLight => {
                show_signal_aspect_internal(self, "Ks1Flashing", cfg)
            }
            SCILSMain::Ks2 => show_signal_aspect_internal(self, "Ks2", cfg),
            SCILSMain::Ks2WithAdditionalLight => {
                show_signal_aspect_internal(self, "Ks2WithAdditionalLight", cfg)
            }
            SCILSMain::Sh1 => show_signal_aspect_internal(self, "Sh1", cfg),
            SCILSMain::IdLight => show_signal_aspect_internal(self, "IdLight", cfg),
            SCILSMain::Hp0Hv => show_signal_aspect_internal(self, "Hp0Hv", cfg),
            SCILSMain::Hp1 => show_signal_aspect_internal(self, "Hp1", cfg),
            SCILSMain::Hp2 => show_signal_aspect_internal(self, "Hp2", cfg),
            SCILSMain::Vr0 => show_signal_aspect_internal(self, "Vr0", cfg),
            SCILSMain::Vr1 => show_signal_aspect_internal(self, "Vr1", cfg),
            SCILSMain::Vr2 => show_signal_aspect_internal(self, "Vr2", cfg),
            SCILSMain::Off => show_signal_aspect_internal(self, "Off", cfg),
        }
        self.main_aspect = signal_aspect.main();
    }

    pub fn signal_aspect_status(&self) -> SCILSSignalAspect {
        let nationally_specified_information = [0u8; 9];
        let signal_aspect = SCILSSignalAspect::new(
            self.main_aspect,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            nationally_specified_information,
        );
        signal_aspect
    }

    pub fn check_signal(&self, cfg: PinConfig) {
        let signal = self.main_aspect_string.clone();
        println!("Check signal {}", signal);
        if cfg.signals.contains_key(&*signal) {
            let led_values = cfg.signals.get(&*signal).unwrap();
            let mut pc = PiControl::new().unwrap();
            for (index, value) in led_values.iter().enumerate() {
                let pin = cfg.pins_input.get(index).unwrap();
                let var_data = pc.find_variable(&pin);
                let res = pc.read(var_data.i16uAddress.into(), 1);
                println!("{}: {:?}", pin, res);
                if res.iter().all(|&v| v == 0)  {
                    println!("NO INPUT SIGNAL FOUND, TRY TO USE THE BACKUP LINE!");
                    let backup_pin = cfg.pins_output_backup.get(index).unwrap();
                    set_pin_value(&mut pc, value, &backup_pin);
                }
            }
        }
    }
}
