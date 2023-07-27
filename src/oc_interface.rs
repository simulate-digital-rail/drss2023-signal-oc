use sci_rs::scils::{SCILSMain, SCILSSignalAspect};
use picontrol::PiControl;
use picontrol::bindings::SPIValue;
use crate::io_config::PinConfig;

pub fn show_signal_aspect(signal_aspect: SCILSSignalAspect, cfg: PinConfig) {

    match signal_aspect.main() {
        SCILSMain::Hp0 => { show_signal_aspect(cfg, "Hp0");}
        SCILSMain::Hp0PlusSh1 => { show_signal_aspect(cfg, "Hp0PlusSh1");}
        SCILSMain::Hp0WithDrivingIndicator => {}
        SCILSMain::Ks1 => { show_signal_aspect(cfg, "Ks1")}
        SCILSMain::Ks1Flashing => { show_signal_aspect(cfg, "Ks1Flashing");}
        SCILSMain::Ks1FlashingWithAdditionalLight => { show_signal_aspect(cfg, "Ks1Flashing")}
        SCILSMain::Ks2 => { show_signal_aspect(cfg, "Ks2")}
        SCILSMain::Ks2WithAdditionalLight => { show_signal_aspect(cfg, "Ks2WithAdditionalLight")}
        SCILSMain::Sh1 => { show_signal_aspect(cfg, "Sh1")}
        SCILSMain::IdLight => { show_signal_aspect(cfg, "IdLight")}
        SCILSMain::Hp0Hv => { show_signal_aspect(cfg, "Hp0Hv")}
        SCILSMain::Hp1 => { show_signal_aspect(cfg, "Hp1")}
        SCILSMain::Hp2 => { show_signal_aspect(cfg, "Hp2")}
        SCILSMain::Vr0 => { show_signal_aspect(cfg, "Vr0")}
        SCILSMain::Vr1 => { show_signal_aspect(cfg, "Vr1")}
        SCILSMain::Vr2 => { show_signal_aspect(cfg, "Vr2")}
        SCILSMain::Off => {
            show_signal_aspect(cfg, "Off");
        }
    }

    fn show_signal_aspect(cfg: PinConfig, signal: &str){
        println!("Signal shows {}", signal);
        if cfg.signals.contains_key(signal){
            let led_values = cfg.signals.get(signal).unwrap();
            let pc = PiControl::new().unwrap();
            for (index, value) in led_values.iter().enumerate(){
                let pin = cfg.pins.get(index).unwrap();
                println!("PIN: {}, VALUE: {}", pin, value);

                let var_data = pc.find_variable(&pin);
                let mut val = SPIValue {
                    i16uAddress: var_data.i16uAddress,
                    i8uBit: var_data.i8uBit,
                    i8uValue: *value
                };
                pc.set_bit_value(&mut val);
            }
        }else{
            eprintln!("NO CONFIG FOUND FOR SCI SIGNAL {}", signal)
        }
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


