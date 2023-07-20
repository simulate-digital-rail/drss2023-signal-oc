#![recursion_limit = "256"]

pub mod rasta_grpc {
    tonic::include_proto!("sci");
}

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{io, thread};

use rasta_grpc::rasta_client::RastaClient;
use rasta_grpc::SciPacket;
use sci_rs::scils::{SCILSBrightness, SCILSMain, SCILSSignalAspect};
use sci_rs::{ProtocolType, SCIMessageType, SCITelegram, SCIVersionCheckResult};
use tokio::time;
use tonic::Request;

const SEND_INTERVAL_MS: u64 = 500;
const SCI_LS_VERSION: u8 = 0x03;

#[derive(PartialEq, Clone, Debug)]
enum OCConnectionState {
    VersionRequestSent,   // version request sent, awaiting version response
    StatusRequestSent,    // version response received, status request sent
    StatusBeginReceived,  // status begin received, awaiting signal aspect
    SignalAspectReceived, // signal aspect received, awaiting brightness
    BrightnessReceived,   // status transmission, awaiting status finish
    Connected,            // handshake completed successfully
    Terminated,           // closed because of errors
}

struct OCState {
    confirmed_signal_aspect: Option<SCILSSignalAspect>,
    confirmed_brightness: Option<SCILSBrightness>,
    conn_state: OCConnectionState,
}

fn create_telegram_from_main(main: SCILSMain) -> SCITelegram {
    let signal_aspect = SCILSSignalAspect::new(
        main,
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        [0u8; 9],
    );
    SCITelegram::scils_show_signal_aspect("C", "S", signal_aspect)
}

// MD5 (16 bytes)
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
        println!("Received version response telegram");
        let remote_check_result: SCIVersionCheckResult =
            sci_telegram.payload.data[0].try_into().unwrap();
        let remote_version = sci_telegram.payload.data[1];
        let checksum_len: usize = sci_telegram.payload.data[2].into();
        let checksum = &sci_telegram.payload.data[3..(3 + checksum_len)];
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
            return Some(SCITelegram::release_for_maintenance(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
            ));
        }
    } else if sci_telegram.message_type == SCIMessageType::sci_status_begin()
        && state.conn_state == OCConnectionState::StatusRequestSent
    {
        println!("Received status begin telegram");
        state.conn_state = OCConnectionState::StatusBeginReceived;
    } else if sci_telegram.message_type == SCIMessageType::sci_status_finish()
        && state.conn_state == OCConnectionState::BrightnessReceived
    {
        println!("Received status finish telegram");
        state.conn_state = OCConnectionState::Connected;
    } else {
        println!("The received packet of type {} is either unrecognized or was received in the wrong order during handshake!", sci_telegram.message_type.try_as_sci_message_type().unwrap_or("UNKNOWN"));
        state.conn_state = OCConnectionState::Terminated;
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

    let oc_state = OCState {
        confirmed_signal_aspect: None,
        confirmed_brightness: None,
        conn_state: OCConnectionState::VersionRequestSent,
    };

    let lock_state = RwLock::new(oc_state);
    let input_lock_state = Arc::new(lock_state);
    let receive_lock_state = input_lock_state.clone();
    let send_lock_state = input_lock_state.clone();

    // begin handshake with sending a version request
    let send_queue: VecDeque<SCITelegram> = VecDeque::from([SCITelegram::version_request(
        ProtocolType::SCIProtocolLS,
        "C",
        "S",
        SCI_LS_VERSION,
    )]);

    let lock_queue = RwLock::new(send_queue);
    let input_lock_queue = Arc::new(lock_queue);
    let receive_lock_queue = input_lock_queue.clone();
    let send_lock_queue = input_lock_queue.clone();

    let mut input_string = String::new();
    thread::spawn(move || loop {
        input_string.clear();
        io::stdin().read_line(&mut input_string).unwrap();
        let locked_oc_state = input_lock_state.read().unwrap();
        let mut locked_send_queue = input_lock_queue.write().unwrap();
        if locked_oc_state.conn_state == OCConnectionState::Connected {
            if input_string.trim() == "Ks1"
                && locked_oc_state
                    .confirmed_signal_aspect
                    .as_ref()
                    .unwrap()
                    .main()
                    != SCILSMain::Ks1
            {
                locked_send_queue.push_back(create_telegram_from_main(SCILSMain::Ks1));
            } else if input_string.trim() == "Ks2"
                && locked_oc_state
                    .confirmed_signal_aspect
                    .as_ref()
                    .unwrap()
                    .main()
                    != SCILSMain::Ks2
            {
                locked_send_queue.push_back(create_telegram_from_main(SCILSMain::Ks2));
            } else if input_string.trim() == "Off"
                && locked_oc_state
                    .confirmed_signal_aspect
                    .as_ref()
                    .unwrap()
                    .main()
                    != SCILSMain::Off
            {
                locked_send_queue.push_back(create_telegram_from_main(SCILSMain::Off));
            }
        } else {
            println!("Cannot send signal aspect because OC is not connected!");
        }
        thread::sleep(Duration::from_millis(1000));
    });

    let outbound = async_stream::stream! {
        let mut interval = time::interval(Duration::from_millis(SEND_INTERVAL_MS));
        while let time = interval.tick().await {
            let mut message = Vec::new();
            {
                let mut locked_oc_state = send_lock_state.write().unwrap();
                let mut locked_send_queue = send_lock_queue.write().unwrap();
                if locked_oc_state.conn_state == OCConnectionState::Terminated {
                    break; // TODO: after refactoring to server, we should not shutdown here
                }
                if let Some(telegram) = locked_send_queue.pop_front() {
                    if telegram.message_type == SCIMessageType::release_for_maintenance() {
                        locked_oc_state.conn_state = OCConnectionState::Terminated;
                    }
                    message = telegram.into();
                }
            }
            if message.len() > 0 {
                yield SciPacket {message};
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

        let mut locked_oc_state = receive_lock_state.write().unwrap();
        if let Some(sci_response) = handle_incoming_telegram(sci_telegram, &mut locked_oc_state) {
            let mut locked_send_queue = receive_lock_queue.write().unwrap();
            locked_send_queue.push_back(sci_response);
        }
    }

    Ok(())
}
