mod oc_interface;

pub mod rasta_grpc {
    tonic::include_proto!("sci");
}

use std::pin::Pin;

use futures_core::Stream;
use futures_util::StreamExt;
use rasta_grpc::rasta_server::{Rasta, RastaServer};
use rasta_grpc::SciPacket;
use sci_rs::scils::SCILSSignalAspect;
use sci_rs::{SCIMessageType, SCITelegram};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Debug)]
struct RastaService;

#[tonic::async_trait]
impl Rasta for RastaService {
    type StreamStream = Pin<Box<dyn Stream<Item = Result<SciPacket, Status>> + Send + 'static>>;

    async fn stream(
        &self,
        request: Request<tonic::Streaming<SciPacket>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        let mut stream = request.into_inner();

        let output = async_stream::try_stream! {
            while let Some(sci_packet) = stream.next().await {
                let sci_packet = sci_packet?;
                let sci_telegram = sci_packet.message.as_slice().try_into()
                    .unwrap_or_else(|e| panic!("Could not convert packet into SCITelegram: {:?}", e));
                if let Some(sci_response) = handle_incoming_telegram(sci_telegram) {
                    yield SciPacket {
                        message: sci_response.into()
                    };
                }
            }
        };

        Ok(Response::new(Box::pin(output) as Self::StreamStream))
    }
}

fn handle_incoming_telegram(sci_telegram: SCITelegram) -> Option<SCITelegram> {
    if sci_telegram.message_type == SCIMessageType::scils_show_signal_aspect() {
        let status_change =
            SCILSSignalAspect::try_from(sci_telegram.payload.data.as_slice()).unwrap();
        println!(
            "Received show signal aspect telegram: changing main to {:?} (from {}, to {})",
            status_change.main(),
            sci_telegram.sender,
            sci_telegram.receiver
        );
        println!("Should show signal aspect");
        oc_interface::show_signal_aspect(status_change);
        Some(SCITelegram::scils_signal_aspect_status(
            &*sci_telegram.receiver,
            &*sci_telegram.sender,
            oc_interface::signal_aspect_status(),
        ))
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_ip_addr = std::env::args().nth(1).unwrap();
    let server_port = std::env::args().nth(2).unwrap();
    let addr = format!("{}:{}", server_ip_addr, server_port)
        .parse()
        .unwrap();

    println!("Starting receiver!");
    let rasta_service = RastaService;
    let server = RastaServer::new(rasta_service);
    Server::builder().add_service(server).serve(addr).await?;

    Ok(())
}
