use sci_rs::scils::{SCILSMain, SCILSSignalAspect};
use picontrol::PiControl;
use picontrol::bindings::SPIValue;

pub fn show_signal_aspect(signal_aspect: SCILSSignalAspect) {
    println!("Show signal aspect called");


    match signal_aspect.main() {
        SCILSMain::Hp0 => {}
        SCILSMain::Hp0PlusSh1 => {}
        SCILSMain::Hp0WithDrivingIndicator => {}
        SCILSMain::Ks1 => {
            println!("Signal shows Ks1");
            let led_values: [u8; 4] = [1,1,1,1];
            do_run(led_values)
        }
        SCILSMain::Ks1Flashing => {}
        SCILSMain::Ks1FlashingWithAdditionalLight => {}
        SCILSMain::Ks2 => {
            println!("Signal shows Ks2");
            let led_values: [u8; 4] = [0,0,1,1];
            do_run(led_values)
        }
        SCILSMain::Ks2WithAdditionalLight => {}
        SCILSMain::Sh1 => {}
        SCILSMain::IdLight => {}
        SCILSMain::Hp0Hv => {}
        SCILSMain::Hp1 => {}
        SCILSMain::Hp2 => {}
        SCILSMain::Vr0 => {}
        SCILSMain::Vr1 => {}
        SCILSMain::Vr2 => {}
        SCILSMain::Off => {
            println!("OFF");
            let led_values: [u8; 4] = [0,0,0,0];
            do_run(led_values)
        }
    }

    fn do_run(led_values: [u8;4]){
        let pc = PiControl::new().unwrap();
        for led_value in led_values.iter().enumerate(){
            let (i, x): (usize, &u8) = led_value;
            let index = i+1;
            println!("index {:?},", index);
            let led_pin = format!("O_{index}");
            println!("led_pin {:?},", led_pin);
            let var_data = pc.find_variable(&led_pin);
            println!("VAR_DATA {:?},", var_data);
            let mut val = SPIValue {
                i16uAddress: var_data.i16uAddress,
                i8uBit: var_data.i8uBit,
                i8uValue: *x
            };
            pc.set_bit_value(&mut val);
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


