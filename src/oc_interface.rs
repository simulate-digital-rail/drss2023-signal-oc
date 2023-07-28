use crate::io_config::PinConfig;
use picontrol::bindings::SPIValue;
use picontrol::PiControl;
use sci_rs::scils::{SCILSMain, SCILSSignalAspect};

fn show_signal_aspect(signal: &str, cfg: PinConfig) {
    println!("Signal shows {}", signal);
    if cfg.signals.contains_key(signal) {
        let led_values = cfg.signals.get(signal).unwrap();
        let pc = PiControl::new().unwrap();
        for (index, value) in led_values.iter().enumerate() {
            let pin = cfg.pins.get(index).unwrap();
            println!("PIN: {}, VALUE: {}", pin, value);

            let var_data = pc.find_variable(&pin);
            let mut val = SPIValue {
                i16uAddress: var_data.i16uAddress,
                i8uBit: var_data.i8uBit,
                i8uValue: *value,
            };
            pc.set_bit_value(&mut val);
        }
    } else {
        eprintln!("NO CONFIG FOUND FOR SCI SIGNAL {}", signal)
    }
}

pub fn show_signal_aspect(signal_aspect: SCILSSignalAspect, cfg: PinConfig) {
    match signal_aspect.main() {
        SCILSMain::Hp0 => show_signal_aspect("Hp0", cfg),
        SCILSMain::Hp0PlusSh1 => show_signal_aspect("Hp0PlusSh1", cfg),
        SCILSMain::Hp0WithDrivingIndicator => {}
        SCILSMain::Ks1 => show_signal_aspect("Ks1", cfg),
        SCILSMain::Ks1Flashing => show_signal_aspect("Ks1Flashing", cfg),
        SCILSMain::Ks1FlashingWithAdditionalLight => show_signal_aspect("Ks1Flashing", cfg),
        SCILSMain::Ks2 => show_signal_aspect("Ks2", cfg),
        SCILSMain::Ks2WithAdditionalLight => show_signal_aspect("Ks2WithAdditionalLight", cfg),
        SCILSMain::Sh1 => show_signal_aspect("Sh1", cfg),
        SCILSMain::IdLight => show_signal_aspect("IdLight", cfg),
        SCILSMain::Hp0Hv => show_signal_aspect("Hp0Hv", cfg),
        SCILSMain::Hp1 => show_signal_aspect("Hp1", cfg),
        SCILSMain::Hp2 => show_signal_aspect("Hp2", cfg),
        SCILSMain::Vr0 => show_signal_aspect("Vr0", cfg),
        SCILSMain::Vr1 => show_signal_aspect("Vr1", cfg),
        SCILSMain::Vr2 => show_signal_aspect("Vr2", cfg),
        SCILSMain::Off => show_signal_aspect("Off", cfg),
    }
}

pub fn signal_aspect_status() -> SCILSSignalAspect {
    let nationally_specified_information = [0u8; 9];
    //TODO returning actual status
    let signal_aspect = SCILSSignalAspect::new(
        SCILSMain::Ks1,
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
