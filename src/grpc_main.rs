use drss_2023_object_controller::rasta_client::RastaClient;
use drss_2023_object_controller::SciPacket;
use futures_util::stream;

pub mod drss_2023_object_controller {
    tonic::include_proto!("sci");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = RastaClient::connect("http://[::1]:50051").await?;

    let packets = vec![SciPacket { message: vec![42] }];

    let response = client.stream(stream::iter(packets.clone())).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
