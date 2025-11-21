use log::{error, info, warn};
mod byte_handlers;
mod globals;
mod handlers;
mod packet;
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
                tokio::spawn(async move { handle_client(&mut socket).await });
            }
            Err(e) => warn!("couldn't get client: {:?}", e),
        }
    }
}
