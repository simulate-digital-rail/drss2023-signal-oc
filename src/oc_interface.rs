use sci_rs::scils::{SCILSMain, SCILSSignalAspect};

pub fn show_signal_aspect(signal_aspect: SCILSSignalAspect) {
    println!("Show signal aspect called");
    match signal_aspect.main() {
        SCILSMain::Hp0 => {}
        SCILSMain::Hp0PlusSh1 => {}
        SCILSMain::Hp0WithDrivingIndicator => {}
        SCILSMain::Ks1 => {
            println!("Signal shows Ks1")
        }
        SCILSMain::Ks1Flashing => {}
        SCILSMain::Ks1FlashingWithAdditionalLight => {}
        SCILSMain::Ks2 => {
            println!("Signal shows Ks2")
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
            println!("Signal turned off")
        }
    }
}

pub fn signal_aspect_status() -> SCILSSignalAspect {
    let nationally_specified_information = [0u8; 9];
    //TODO returning actual status
    let signal_aspect = SCILSSignalAspect::new(
        SCILSMain::Ks2,
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
