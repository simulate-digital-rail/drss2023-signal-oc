pub mod drss_2023_object_controller {
    tonic::include_proto!("sci");
}

use drss_2023_object_controller::rasta_client::RastaClient;
use drss_2023_object_controller::SciPacket;
use futures_util::stream;
use sci_rs::scils::SCILSBrightness;
use sci_rs::{SCIMessageType, SCITelegram};

fn handle_incoming_telegram(sci_telegram: SCITelegram) -> Option<SCITelegram> {
    if sci_telegram.message_type == SCIMessageType::scils_brightness_status() {
        let changed_luminosity = SCILSBrightness::try_from(sci_telegram.payload.data[0]).unwrap();
        println!(
            "Received brightness status telegram: changed to {:?} (from {}, to {})",
            changed_luminosity, sci_telegram.sender, sci_telegram.receiver
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

    let packets = vec![SciPacket {
        message: SCITelegram::scils_change_brightness("C", "S", SCILSBrightness::Night).into(),
    }];

    let response = client.stream(stream::iter(packets.clone())).await?;
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
