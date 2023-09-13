mod io_config;
mod oc_interface;

pub mod rasta_grpc {
    tonic::include_proto!("sci");
}

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use io_config::PinConfig;
use md5;
use rasta_grpc::rasta_client::RastaClient;
use rasta_grpc::SciPacket;
use sci_rs::scils::{SCILSBrightness, SCILSMain, SCILSSignalAspect};
use sci_rs::{ProtocolType, SCIMessageType, SCITelegram, SCIVersionCheckResult};
use tokio::time;
use tonic::Request;
use clokwerk::{Scheduler, TimeUnits};
use clokwerk::Interval::*;

const SEND_INTERVAL_MS: u64 = 500;
const SCI_LS_VERSION: u8 = 0x03;

#[derive(PartialEq, Clone, Debug)]
enum InterlockingConnectionState {
    Unconnected,
    VersionResponseSent,
    Connected,
    Terminated,
}

fn check_version(sender_version: u8) -> SCIVersionCheckResult {
    if sender_version == SCI_LS_VERSION {
        SCIVersionCheckResult::VersionsAreEqual
    } else {
        SCIVersionCheckResult::VersionsAreNotEqual
    }
}

// MD5 (16 bytes)
fn compute_checksum(pseudo_telegram: SCITelegram) -> Vec<u8> {
    md5::compute::<Vec<u8>>(pseudo_telegram.into()).to_vec()
}

fn handle_incoming_telegram(
    oc : &mut oc_interface::OC,
    sci_telegram: SCITelegram,
    state: &mut InterlockingConnectionState,
    io_cfg: PinConfig,
) -> Vec<SCITelegram> {
    if sci_telegram.message_type == SCIMessageType::scils_show_signal_aspect() {
        let status_change =
            SCILSSignalAspect::try_from(sci_telegram.payload.data.as_slice()).unwrap();
        println!(
            "Received show signal aspect telegram: changing main to {:?}",
            status_change.main()
        );
        oc.show_signal_aspect(status_change, io_cfg.clone());
        vec![SCITelegram::scils_signal_aspect_status(
            &*sci_telegram.receiver,
            &*sci_telegram.sender,
            oc.signal_aspect_status(),
        )]
    } else if sci_telegram.message_type == SCIMessageType::scils_change_brightness() {
        println!(
            "Interlocking commanded to change brightness, but this is not implemented for this OC!"
        );
        vec![]
    } else if sci_telegram.message_type == SCIMessageType::sci_version_request() {
        let check_result = check_version(sci_telegram.payload.data[0]);
        *state = InterlockingConnectionState::VersionResponseSent;
        if check_result == SCIVersionCheckResult::VersionsAreEqual {
            println!("Received version request - sending version response telegram -> version check successful");
            let checksum = compute_checksum(SCITelegram::version_response(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
                SCI_LS_VERSION,
                check_result,
                &[0],
            ));
            return vec![SCITelegram::version_response(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
                SCI_LS_VERSION,
                check_result,
                checksum.as_slice(),
            )];
        } else {
            println!("Received version request - sending version response telegram -> version check failed");
            vec![SCITelegram::version_response(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
                SCI_LS_VERSION,
                check_result,
                &[0],
            )]
        }
    } else if sci_telegram.message_type == SCIMessageType::sci_status_request() {
        println!("Received status request - sending status telegrams");
        *state = InterlockingConnectionState::Connected;
        vec![
            SCITelegram::status_begin(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
            ),
            SCITelegram::scils_signal_aspect_status(
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
                oc.signal_aspect_status(),
            ),
            SCITelegram::scils_brightness_status(
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
                SCILSBrightness::Day,
            ),
            SCITelegram::status_finish(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
            ),
        ]
    } else if sci_telegram.message_type == SCIMessageType::sci_release_for_maintenance()
        || sci_telegram.message_type == SCIMessageType::sci_close()
    {
        println!("Received release for maintenance or close - shutting down now!");
        *state = InterlockingConnectionState::Terminated;
        vec![]
    } else {
        println!(
            "Cannot handle received telegram of type {}!",
            sci_telegram
                .message_type
                .try_as_sci_message_type()
                .unwrap_or("UNKNOWN")
        );
        vec![]
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let io_cfg = io_config::get_config(3);

    let most_restrictive_aspect = SCILSSignalAspect::new(
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
        [0u8; 9],
    );

    let bridge_ip_addr = std::env::args().nth(1).unwrap();
    let bridge_port = std::env::args().nth(2).unwrap();

    let mut client =
        RastaClient::connect(format!("http://{}:{}", bridge_ip_addr, bridge_port)).await?;
    println!("OC software started!");

    let mut oc = oc_interface::OC { main_aspect: Default::default()};
    let mut scheduler = Scheduler::new();
    scheduler.every(0.1.seconds()).run(|| oc.check_signal(io_cfg.clone()));

    // establish initial state of outputs
    oc.show_signal_aspect(most_restrictive_aspect.clone(), io_cfg.clone());

    let send_queue: VecDeque<SCITelegram> = VecDeque::new();
    let lock_queue = RwLock::new(send_queue);
    let receive_lock_queue = Arc::new(lock_queue);
    let send_lock_queue = receive_lock_queue.clone();

    let mut conn_state = InterlockingConnectionState::Unconnected;

    let outbound = async_stream::stream! {
        let mut interval = time::interval(Duration::from_millis(SEND_INTERVAL_MS));
        while let time = interval.tick().await {
            let mut message = Vec::new();
            {
                let mut locked_send_queue = send_lock_queue.write().unwrap();
                if let Some(telegram) = locked_send_queue.pop_front() {
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
        let mut locked_send_queue = receive_lock_queue.write().unwrap();
        for sci_response in handle_incoming_telegram(&mut oc,sci_telegram, &mut conn_state, io_cfg.clone())
        {
            locked_send_queue.push_back(sci_response);
        }
        if conn_state == InterlockingConnectionState::Terminated {
            break;
        }
    }

    // fallback when connection is interrupted
    oc.show_signal_aspect(most_restrictive_aspect, io_cfg.clone());

    Ok(())
}
