use sci_rs::scils::{SCILSMain, SCILSSignalAspect};

pub fn show_signal_aspect(signal_aspect: SCILSSignalAspect){

}

pub fn signal_aspect_status() -> SCILSSignalAspect {
    let signal_aspect = SCILSSignalAspect::new(
        SCILSMain::Hp0,
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        nationally_specified_information
    );
    signal_aspect
}