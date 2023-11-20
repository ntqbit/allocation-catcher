mod client;
mod ipc;
mod proto;

use bytes::BytesMut;
use clap::Command;

use client::{Client, PacketId};
use ipc::IpcClient;
use prost::Message;

fn cli() -> Command {
    Command::new("allocation-catcher")
        .about("Allocation catcher")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(Command::new("ping").about("Ping"))
}

fn main() {
    // TODO: remove unwrap
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("ping", _)) => {
            let mut client = Client::new(IpcClient::connect("allocation-catcher").unwrap());
            let challenge = rand::random();
            let request = proto::PingRequest { num: challenge };
            let mut buf = BytesMut::new();
            request.encode(&mut buf).unwrap();
            let resp = client.request(PacketId::Ping, buf.freeze());
            let ping_response = proto::PingResponse::decode(resp).unwrap();
            if ping_response.num == challenge {
                println!("Ping-pong! Version: {}", ping_response.version);
            } else {
                println!("Ping failed! Wrong response challenge.");
            }
        }
        Some((_, _)) => unreachable!(),
        None => todo!(),
    }
}
