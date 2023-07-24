
use std::env;
use std::ptr::null;
use sci_rs::scils::{SCILSMain, SCILSSignalAspect};
use picontrol::PiControl;
use picontrol::bindings::SPIValue;

use crate::PinConfig;


pub fn show_signal_aspect(signal_aspect: SCILSSignalAspect, cfg: PinConfig) {

    match signal_aspect.main() {
        SCILSMain::Hp0 => {do_run(cfg, "Hp0");}
        SCILSMain::Hp0PlusSh1 => {do_run(cfg, "Hp0PlusSh1");}
        SCILSMain::Hp0WithDrivingIndicator => {}
        SCILSMain::Ks1 => {do_run(cfg, "Ks1")}
        SCILSMain::Ks1Flashing => { do_run(cfg, "Ks1Flashing");}
        SCILSMain::Ks1FlashingWithAdditionalLight => {do_run(cfg, "Ks1Flashing")}
        SCILSMain::Ks2 => { do_run(cfg, "Ks2")}
        SCILSMain::Ks2WithAdditionalLight => {do_run(cfg, "Ks2WithAdditionalLight")}
        SCILSMain::Sh1 => {do_run(cfg, "Sh1")}
        SCILSMain::IdLight => {do_run(cfg, "IdLight")}
        SCILSMain::Hp0Hv => {do_run(cfg, "Hp0Hv")}
        SCILSMain::Hp1 => {do_run(cfg, "Hp1")}
        SCILSMain::Hp2 => {do_run(cfg, "Hp2")}
        SCILSMain::Vr0 => {do_run(cfg, "Vr0")}
        SCILSMain::Vr1 => {do_run(cfg, "Vr1")}
        SCILSMain::Vr2 => {do_run(cfg, "Vr2")}
        SCILSMain::Off => {
            do_run(cfg, "Off");
        }
    }

    fn do_run(cfg: PinConfig, signal: &str){
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


