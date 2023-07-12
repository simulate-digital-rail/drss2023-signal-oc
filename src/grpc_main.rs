mod oc_interface;

pub mod rasta_grpc {
    tonic::include_proto!("sci");
}

use std::pin::Pin;

use futures_core::Stream;
use futures_util::StreamExt;
use md5;
use rasta_grpc::rasta_server::{Rasta, RastaServer};
use rasta_grpc::SciPacket;
use sci_rs::scils::{SCILSBrightness, SCILSMain, SCILSSignalAspect};
use sci_rs::{ProtocolType, SCIMessageType, SCITelegram, SCIVersionCheckResult};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

const SCI_LS_VERSION: u8 = 0x03;

struct RastaService {
    most_restrictive_aspect: SCILSSignalAspect,
}

#[tonic::async_trait]
impl Rasta for RastaService {
    type StreamStream = Pin<Box<dyn Stream<Item = Result<SciPacket, Status>> + Send + 'static>>;

    async fn stream(
        &self,
        request: Request<tonic::Streaming<SciPacket>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        let mut stream = request.into_inner();
        let cloned_most_restrictive_aspect = self.most_restrictive_aspect.clone();

        let output = async_stream::try_stream! {
            while let Some(sci_packet) = stream.next().await {
                let sci_packet = sci_packet?;
                let sci_telegram = sci_packet.message.as_slice().try_into()
                    .unwrap_or_else(|e| panic!("Could not convert packet into SCITelegram: {:?}", e));
                for response in handle_incoming_telegram(sci_telegram) {
                    yield SciPacket {
                        message: response.into()
                    }
                }
            }

            // fallback when connection is interrupted
            oc_interface::show_signal_aspect(cloned_most_restrictive_aspect);
        };

        Ok(Response::new(Box::pin(output) as Self::StreamStream))
    }
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

fn handle_incoming_telegram(sci_telegram: SCITelegram) -> Vec<SCITelegram> {
    if sci_telegram.message_type == SCIMessageType::scils_show_signal_aspect() {
        let status_change =
            SCILSSignalAspect::try_from(sci_telegram.payload.data.as_slice()).unwrap();
        println!(
            "Received show signal aspect telegram: changing main to {:?} (from {}, to {})",
            status_change.main(),
            sci_telegram.sender,
            sci_telegram.receiver
        );
        oc_interface::show_signal_aspect(status_change);
        vec![SCITelegram::scils_signal_aspect_status(
            &*sci_telegram.receiver,
            &*sci_telegram.sender,
            oc_interface::signal_aspect_status(),
        )]
    } else if sci_telegram.message_type == SCIMessageType::scils_change_brightness() {
        println!(
            "Interlocking commanded to change brightness, but this is not implemented for this OC!"
        );
        vec![]
    } else if sci_telegram.message_type == SCIMessageType::sci_version_request() {
        let check_result = check_version(sci_telegram.payload.data[0]);
        if check_result == SCIVersionCheckResult::VersionsAreEqual {
            println!("Received version request - sending version response telegram -> version check successful");
            let checksum = compute_checksum(SCITelegram::version_response(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
                SCI_LS_VERSION,
                check_version(sci_telegram.payload.data[0]),
                &[0],
            ));
            return vec![SCITelegram::version_response(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
                SCI_LS_VERSION,
                check_version(sci_telegram.payload.data[0]),
                checksum.as_slice(),
            )];
        } else {
            println!("Received version request - sending version response telegram -> version check failed");
            vec![SCITelegram::version_response(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
                SCI_LS_VERSION,
                check_version(sci_telegram.payload.data[0]),
                &[0],
            )]
        }
    } else if sci_telegram.message_type == SCIMessageType::sci_status_request() {
        println!("Received status request - sending status telegrams");
        vec![
            SCITelegram::status_begin(
                ProtocolType::SCIProtocolLS,
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
            ),
            SCITelegram::scils_signal_aspect_status(
                &*sci_telegram.receiver,
                &*sci_telegram.sender,
                oc_interface::signal_aspect_status(),
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

    let server_ip_addr = std::env::args().nth(1).unwrap();
    let server_port = std::env::args().nth(2).unwrap();
    let addr = format!("{}:{}", server_ip_addr, server_port)
        .parse()
        .unwrap();

    // establish initial state of outputs
    oc_interface::show_signal_aspect(most_restrictive_aspect.clone());

    println!("Starting receiver!");
    let rasta_service = RastaService {
        most_restrictive_aspect: most_restrictive_aspect.clone(),
    };
    let server = RastaServer::new(rasta_service);
    Server::builder().add_service(server).serve(addr).await?;

    Ok(())
}
