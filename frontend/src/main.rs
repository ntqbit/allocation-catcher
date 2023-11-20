mod client;
mod ipc;
mod proto;

use std::io;

use clap::{ArgMatches, Command};

use client::{Client, PacketId};
use ipc::IpcClient;

fn cli() -> Command {
    Command::new("allocation-catcher")
        .about("Allocation catcher")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(Command::new("ping").about("Ping"))
}

pub trait RequestSpec: prost::Message {
    const PACKET_ID: PacketId;

    type RESPONSE: prost::Message + Default;
}

impl RequestSpec for proto::PingRequest {
    const PACKET_ID: PacketId = PacketId::Ping;

    type RESPONSE = proto::PingResponse;
}

pub fn send_request<T: RequestSpec>(msg: T) -> io::Result<T::RESPONSE> {
    let mut client = Client::new(IpcClient::connect("allocation-catcher")?);
    let response_bytes = client.request(T::PACKET_ID, msg.encode_to_vec().into())?;
    Ok(<T::RESPONSE as prost::Message>::decode(response_bytes)?)
}

fn run(matches: ArgMatches) -> io::Result<()> {
    match matches.subcommand() {
        Some(("ping", _)) => {
            let challenge = rand::random();
            let req = proto::PingRequest { num: challenge };
            let ping_response = send_request(req)?;
            if ping_response.num == challenge {
                println!("Ping-pong! Version: {}", ping_response.version);
            } else {
                println!("Ping failed! Wrong response challenge.");
            }
        }
        Some((cmd, _)) => panic!("Unhandled command: {}", cmd),
        None => panic!("How to handle None?"), // TODO: handle
    }

    Ok(())
}

fn main() {
    run(cli().get_matches()).unwrap();
}
