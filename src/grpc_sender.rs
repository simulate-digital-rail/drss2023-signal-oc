#![recursion_limit = "256"]

pub mod rasta_grpc {
    tonic::include_proto!("sci");
}

use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{io, thread};

use rasta_grpc::rasta_client::RastaClient;
use rasta_grpc::SciPacket;
use sci_rs::scils::{SCILSMain, SCILSSignalAspect};
use sci_rs::{SCIMessageType, SCITelegram};
use tokio::time;
use tonic::Request;

fn handle_incoming_telegram(sci_telegram: SCITelegram) -> Option<SCITelegram> {
    if sci_telegram.message_type == SCIMessageType::scils_signal_aspect_status() {
        let changed_signal_aspect =
            SCILSSignalAspect::try_from(sci_telegram.payload.data.as_slice()).unwrap();
        println!(
            "Received signal aspect status telegram: main was changed to {:?} (from {}, to {})",
            changed_signal_aspect.main(),
            sci_telegram.sender,
            sci_telegram.receiver
        );
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bridge_ip_addr = std::env::args().nth(1).unwrap();
    let bridge_port = std::env::args().nth(2).unwrap();

    let mut client =
        RastaClient::connect(format!("http://{}:{}", bridge_ip_addr, bridge_port)).await?;
    println!("Sender started!");

    let nationally_specified_information = [0u8; 9];
    let mut current_main_aspect = SCILSMain::Ks2;
    let requested_main_aspect = SCILSMain::Ks2;

    let lock = RwLock::new(requested_main_aspect);
    let send_lock = Arc::new(lock);
    let input_lock = send_lock.clone();

    let mut input_string = String::new();
    thread::spawn(move || loop {
        input_string.clear();
        io::stdin().read_line(&mut input_string).unwrap();
        if input_string.trim() == "Ks1" {
            let mut locked_main_aspect = input_lock.write().unwrap();
            *locked_main_aspect = SCILSMain::Ks1;
        } else if input_string.trim() == "Ks2" {
            let mut locked_main_aspect = input_lock.write().unwrap();
            *locked_main_aspect = SCILSMain::Ks2;
        }
        thread::sleep(Duration::from_millis(1000));
    });

    let outbound = async_stream::stream! {
        let mut interval = time::interval(Duration::from_secs(1));
        while let time = interval.tick().await {
            let new_main_aspect;
            {
                let locked_main_aspect = send_lock.read().unwrap();
                new_main_aspect = *locked_main_aspect;
            }

            if new_main_aspect != current_main_aspect {
                current_main_aspect = new_main_aspect;
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

                println!("sending main={:?} ", current_main_aspect);

                yield SciPacket {
                    message: SCITelegram::scils_show_signal_aspect("C", "S", signal_aspect).into()
                };
            }
        }
    };

    let response = client.stream(Request::new(outbound)).await?;
    let mut inbound = response.into_inner();

    while let Some(sci_packet) = inbound.message().await? {
        let sci_telegram = sci_packet
            .message
            .as_slice()
            .try_into()
            .unwrap_or_else(|e| panic!("Could not convert packet into SCITelegram: {:?}", e));

        if let Some(_sci_response) = handle_incoming_telegram(sci_telegram) {
            // TODO: here, we could send a response back, but currently, we don't
        }
    }

    Ok(())
}
