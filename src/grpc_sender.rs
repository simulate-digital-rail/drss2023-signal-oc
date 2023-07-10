#![recursion_limit = "256"]

pub mod rasta_grpc {
    tonic::include_proto!("sci");
}

use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{io, thread};

use rasta_grpc::rasta_client::RastaClient;
use rasta_grpc::SciPacket;
use sci_rs::scils::{SCILSBrightness, SCILSMain, SCILSSignalAspect};
use sci_rs::{ProtocolType, SCIMessageType, SCITelegram, SCIVersionCheckResult};
use tokio::time;
use tonic::Request;

const SCI_LS_VERSION: u8 = 0x03;

#[derive(PartialEq)]
enum OCConnectionState {
    Unconnected,          // nothing happened so far
    VersionRequestSent,   // version request sent, awaiting version response
    StatusRequestSent,    // version response received, status request sent
    StatusBeginReceived,  // status begin received, awaiting signal aspect
    SignalAspectReceived, // signal aspect received, awaiting brightness
    BrightnessReceived,   // status transmission, awaiting status finish
    Connected,            // handshake completed successfully
}

struct OCState {
    confirmed_signal_aspect: Option<SCILSSignalAspect>,
    confirmed_brightness: Option<SCILSBrightness>,
    conn_state: OCConnectionState,
}

fn check_checksum(
    sci_telegram: &SCITelegram,
    received_checksum: &[u8],
    remote_version: u8,
    remote_check_result: SCIVersionCheckResult,
) -> bool {
    let pseudo_telegram = SCITelegram::version_response(
        ProtocolType::SCIProtocolLS,
        &*sci_telegram.sender,
        &*sci_telegram.receiver,
        remote_version,
        remote_check_result,
        &[0],
    );
    let computed_checksum = md5::compute::<Vec<u8>>(pseudo_telegram.into());
    computed_checksum.as_slice() == received_checksum
}

fn handle_incoming_telegram(sci_telegram: SCITelegram, state: &mut OCState) -> Option<SCITelegram> {
    if sci_telegram.message_type == SCIMessageType::scils_signal_aspect_status()
        && (state.conn_state == OCConnectionState::StatusBeginReceived
            || state.conn_state == OCConnectionState::Connected)
    {
        let new_signal_aspect =
            SCILSSignalAspect::try_from(sci_telegram.payload.data.as_slice()).unwrap();
        println!(
            "Received signal aspect status telegram: main is now {:?}",
            new_signal_aspect.main(),
        );
        state.confirmed_signal_aspect = Some(new_signal_aspect);
        if state.conn_state == OCConnectionState::StatusBeginReceived {
            state.conn_state = OCConnectionState::SignalAspectReceived;
        }
    } else if sci_telegram.message_type == SCIMessageType::scils_brightness_status()
        && (state.conn_state == OCConnectionState::SignalAspectReceived
            || state.conn_state == OCConnectionState::Connected)
    {
        let new_brightness = SCILSBrightness::try_from(sci_telegram.payload.data[0]).unwrap();
        println!(
            "Received brightness status telegram: brightness is now {:?}",
            new_brightness
        );
        state.confirmed_brightness = Some(new_brightness);
        if state.conn_state == OCConnectionState::SignalAspectReceived {
            state.conn_state = OCConnectionState::BrightnessReceived;
        }
    } else if sci_telegram.message_type == SCIMessageType::sci_version_response()
        && state.conn_state == OCConnectionState::VersionRequestSent
    {
        let remote_version = sci_telegram.payload.data[0];
        let remote_check_result: SCIVersionCheckResult =
            sci_telegram.payload.data[1].try_into().unwrap();
        let checksum_len: usize = sci_telegram.payload.data[1].into();
        let checksum = &sci_telegram.payload.data[3..3 + checksum_len];
        if remote_check_result == SCIVersionCheckResult::VersionsAreEqual
            && remote_version == SCI_LS_VERSION
            && checksum_len > 0
            && check_checksum(&sci_telegram, checksum, remote_version, remote_check_result)
        {
            state.conn_state = OCConnectionState::StatusRequestSent;
            return Some(SCITelegram::status_request(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
            ));
        } else {
            println!(
                "Versions are not matching (peer has version {}, we have version {})!",
                remote_version, SCI_LS_VERSION
            );
            // TODO how to end connection?
        }
    } else if sci_telegram.message_type == SCIMessageType::sci_status_begin()
        && state.conn_state == OCConnectionState::StatusRequestSent
    {
        state.conn_state = OCConnectionState::StatusBeginReceived;
    } else if sci_telegram.message_type == SCIMessageType::sci_status_finish()
        && state.conn_state == OCConnectionState::BrightnessReceived
    {
        state.conn_state = OCConnectionState::Connected;
    } else {
        println!("The received packet of type {} is either unrecognized or was received in the wrong order during handshake!", sci_telegram.message_type.try_as_sci_message_type().unwrap_or("UNKNOWN"));
        // TODO optionally end connection?
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

    let mut oc_state = OCState {
        confirmed_signal_aspect: None,
        confirmed_brightness: None,
        conn_state: OCConnectionState::Unconnected,
    };

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

    // TODO we need to begin with sending a version request
    // TODO sending needs to be refactored (take telegrams out of send queue, so they can be added anywhere)
    // TODO only send "show signal aspect" telegrams after connection established
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

        if let Some(_sci_response) = handle_incoming_telegram(sci_telegram, &mut oc_state) {
            // TODO: here, we could send a response back, but currently, we don't
            // TODO: add response to send queue
        }
    }

    Ok(())
}
