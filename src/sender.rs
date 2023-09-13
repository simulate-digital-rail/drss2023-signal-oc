#![recursion_limit = "512"]

pub mod rasta_grpc {
    tonic::include_proto!("sci");
}

use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{io, thread};

use futures_core::Stream;
use futures_util::StreamExt;
use rasta_grpc::rasta_server::Rasta;
use rasta_grpc::SciPacket;
use sci_rs::scils::{SCILSBrightness, SCILSMain, SCILSSignalAspect};
use sci_rs::{ProtocolType, SCICloseReason, SCIMessageType, SCITelegram, SCIVersionCheckResult};
use tokio::time;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use crate::rasta_grpc::rasta_server::RastaServer;

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

struct RastaService {
    signal_main_to_send: Arc<RwLock<Option<SCILSMain>>>,
    brightness_to_send: Arc<RwLock<Option<SCILSBrightness>>>,
}

#[tonic::async_trait]
impl Rasta for RastaService {
    type StreamStream = Pin<Box<dyn Stream<Item = Result<SciPacket, Status>> + Send + 'static>>;

    async fn stream(
        &self,
        request: Request<tonic::Streaming<SciPacket>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        let mut stream = request.into_inner();

        let mut oc_state = OCState {
            confirmed_signal_aspect: None,
            confirmed_brightness: None,
            conn_state: OCConnectionState::VersionRequestSent,
        };

        let cloned_signal_main_to_send = self.signal_main_to_send.clone();
        let cloned_brightness_to_send = self.brightness_to_send.clone();

        let output = async_stream::try_stream! {
            // begin handshake with sending a version request
            yield SciPacket {
                message: SCITelegram::version_request(ProtocolType::SCIProtocolLS, "C", "S",SCI_LS_VERSION).into()
            };

            while let Some(sci_packet) = stream.next().await {
                let sci_packet = sci_packet?;
                let sci_telegram = sci_packet.message.as_slice().try_into()
                    .unwrap_or_else(|e| panic!("Could not convert packet into SCITelegram: {:?}", e));
                if let Some(sci_response) = handle_incoming_telegram(sci_telegram, &mut oc_state) {
                    yield SciPacket {
                        message: sci_response.into()
                    };
                    if oc_state.conn_state == OCConnectionState::Terminated {
                        break;
                    }
                }
                // if connected, wait until user sets a signal aspect, then send it
                if oc_state.conn_state != OCConnectionState::Connected {
                    continue;
                }
                let mut interval = time::interval(Duration::from_millis(SEND_INTERVAL_MS));
                let mut signal_aspect_telegram = None;
                let mut brightness_telegram = None;
                while let time = interval.tick().await {
                    let mut locked_signal_main = cloned_signal_main_to_send.write().unwrap();
                    if let Some(signal_main) = *locked_signal_main {
                        *locked_signal_main = None;
                        if signal_main != oc_state.confirmed_signal_aspect.clone().unwrap().main() {
                            signal_aspect_telegram = Some(create_telegram_from_main(signal_main));
                            break;
                        }
                    }
                    let mut locked_brightness = cloned_brightness_to_send.write().unwrap();
                    if let Some(brightness) = *locked_brightness {
                        *locked_brightness = None;
                        if brightness != oc_state.confirmed_brightness.clone().unwrap() {
                            brightness_telegram = Some(create_telegram_from_brightness(brightness));
                            break;
                        }
                    }
                }
                if let Some(telegram) = signal_aspect_telegram {
                    yield SciPacket {
                        message: telegram.into()
                    };
                }
                if let Some(telegram) = brightness_telegram {
                    yield SciPacket {
                        message: telegram.into()
                    };
                }
            }
        };

        Ok(Response::new(Box::pin(output) as Self::StreamStream))
    }
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

fn create_telegram_from_brightness(brightness: SCILSBrightness) -> SCITelegram {
    SCITelegram::scils_change_brightness("C", "S", brightness)
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
            state.conn_state = OCConnectionState::Terminated;
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
        return Some(SCITelegram::close(
            ProtocolType::SCIProtocolLS,
            &*sci_telegram.receiver,
            &*sci_telegram.sender,
            SCICloseReason::ProtocolError,
        ));
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_ip_addr = std::env::args().nth(1).unwrap();
    let server_port = std::env::args().nth(2).unwrap();
    let addr = format!("{}:{}", server_ip_addr, server_port)
        .parse()
        .unwrap();

    let signal_main_to_send = None;
    let signal_main_lock = RwLock::new(signal_main_to_send);
    let signal_main_lock_send = Arc::new(signal_main_lock);
    let signal_main_lock_input = signal_main_lock_send.clone();

    let brightness_to_send = None;
    let brightness_lock = RwLock::new(brightness_to_send);
    let brightness_lock_send = Arc::new(brightness_lock);
    let brightness_lock_input = brightness_lock_send.clone();

    let mut input_string = String::new();
    thread::spawn(move || loop {
        input_string.clear();
        io::stdin().read_line(&mut input_string).unwrap();
        {
            let mut locked_signal_main = signal_main_lock_input.write().unwrap();
            let mut locked_brightness = brightness_lock_input.write().unwrap();
            if input_string.trim() == "Ks1" {
                *locked_signal_main = Some(SCILSMain::Ks1);
            } else if input_string.trim() == "Ks2" {
                *locked_signal_main = Some(SCILSMain::Ks2);
            } else if input_string.trim() == "Off" {
                *locked_signal_main = Some(SCILSMain::Off);
            } else if input_string.trim() == "Day" {
                *locked_brightness = Some(SCILSBrightness::Day);
            } else if input_string.trim() == "Night" {
                *locked_brightness = Some(SCILSBrightness::Night);
            }
        }
        thread::sleep(Duration::from_millis(1000));
    });

    println!("Starting interlocking!");
    let rasta_service = RastaService {
        signal_main_to_send: signal_main_lock_send,
        brightness_to_send: brightness_lock_send,
    };
    let server = RastaServer::new(rasta_service);
    Server::builder().add_service(server).serve(addr).await?;

    Ok(())
}
