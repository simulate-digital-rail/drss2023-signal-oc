use rasta_rs::RastaConnection;
use sci_rs::SCITelegram;
use sci_rs::scils::{SCILSBrightness, SCILSMain, SCILSSignalAspect};
use sci_rs::SCICommand;
use sci_rs::SCIConnection;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::{io, thread};
use std::time::Duration;

fn main() {
    let addr: SocketAddr = "127.0.0.1:8888".parse().unwrap();
    let conn = RastaConnection::try_new(addr, 42).unwrap();
    let sci_name_rasta_id_mapping = HashMap::from([("C".to_string(), 42), ("S".to_string(), 1337)]);
    let mut sender =
        SCIConnection::try_new(conn, "C".to_string(), sci_name_rasta_id_mapping).unwrap();

    let target_luminosity = SCILSBrightness::Day;
    let mut current_luminosity = SCILSBrightness::Night;

    let target_main_aspect = SCILSMain::Ks1;
    let mut current_main_aspect = SCILSMain::Ks1;


    let lock = RwLock::new(target_main_aspect);
    let send_lock = Arc::new(lock);
    let input_lock = send_lock.clone();


    let nationally_specified_information = [0u8;9];

    let mut input_string = String::new();
    thread::spawn(move || loop {
        input_string.clear();
        io::stdin().read_line(&mut input_string).unwrap();
        if input_string.trim() == "Ks1" {
            let mut locked_main_aspect = input_lock.write().unwrap();
            *locked_main_aspect = SCILSMain::Ks1;
        }
        else if input_string.trim() == "Ks2" {
            let mut locked_main_aspect = input_lock.write().unwrap();
            *locked_main_aspect = SCILSMain::Ks2;
        }


        /*
                {
                    let mut locked_main_aspect = input_lock.write().unwrap();
                    *locked_main_aspect = if *locked_main_aspect == SCILSBrightness::Day {
                        SCILSBrightness::Night
                    } else {
                        SCILSBrightness::Day
                    };
                    println!("ts_input: {:?} ", *locked_luminosity);

                }
        */
        thread::sleep(Duration::from_millis(1000));
    });



    /*

    SCITelegram::scils_show_signal_aspect("C","S",signal_aspect);
     */
    sender
        .run("S", |_data| {
            let locked_main_aspect = send_lock.read().unwrap();
            //println!("ts_sending: {:?} ", locked_main_aspect);
            if current_main_aspect != *locked_main_aspect {
                println!("sending telegram now");
                if *locked_main_aspect == SCILSMain::Ks1 { println!("ts_sending Ks1")}
                if *locked_main_aspect == SCILSMain::Ks2 { println!("ts_sending Ks2")}
                current_main_aspect = *locked_main_aspect;

                let signal_aspect = SCILSSignalAspect::new(
                    current_main_aspect,
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
                return SCICommand::Telegram(SCITelegram::scils_show_signal_aspect(
                    "C",
                    "S",
                    signal_aspect,
                ));
                /*
                return SCICommand::Telegram(SCITelegram::scils_change_brightness(
                    "C",
                    "S",
                    *locked_luminosity,
                ));*/
            }
            SCICommand::Wait
        })
        .unwrap();



    println!("Getting here ?");
}