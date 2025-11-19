use log::*;

mod byte_handlers;
mod handlers;
mod varnums;

use handlers::handle_client;
use tokio::io;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    env_logger::init();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:25565").await?;

    info! {"starting minecraft server"};
    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                info! {"{:?}", socket};
                if let Err(e) = handle_client(&mut socket).await {
                    info! {"error: {:?}", e};
                }
            }
            Err(e) => info!("couldn't get client: {:?}", e),
        }
    }
}
