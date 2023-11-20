mod client;
mod ipc;
mod proto;

use std::io;

use clap::{ArgMatches, Command};

use client::{Client, RequestSpec};
use ipc::IpcClient;

fn cli() -> Command {
    Command::new("allocation-catcher")
        .about("Allocation catcher")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(Command::new("ping").about("Ping"))
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
        Some(("clear", _)) => {
            send_request(proto::ClearStorageRequest {})?;
            println!("Done!");
        }
        Some(("setcfg", _)) => {
            send_request(proto::SetConfigurationRequest {
                configuration: Some(proto::Configuration {
                    stack_trace_offset: 0x10,
                    stack_trace_size: 5,
                }),
            })?;
            println!("Done!");
        }
        Some(("getcfg", _)) => {
            let resp = send_request(proto::GetConfigurationRequest {})?;
            println!("Configuration: {:#?}", resp.configuration);
        }
        Some(("dump", _)) => {
            let resp = send_request(proto::FindRequest {
                records: vec![proto::FindRecord {
                    id: 0,
                    filter: None,
                }],
            })?;
            assert_eq!(resp.allocations.len(), 1);

            let allocations = &resp.allocations.first().unwrap().allocations;

            for allocation in allocations {
                println!(
                    "Allocation: [base=0x{:X}, size=0x{:X}]",
                    allocation.base_address, allocation.size
                );
            }
        }
        Some(("find", _)) => {
            let resp = send_request(proto::FindRequest {
                records: vec![proto::FindRecord {
                    id: 0,
                    filter: Some(proto::Filter {
                        location: Some(proto::filter::Location::Address(0x100)),
                    }),
                }],
            })?;

            let total_allocations = resp
                .allocations
                .iter()
                .fold(0, |acc, x| acc + x.allocations.len());
            println!("Found {} allocations.", total_allocations);
        }
        Some(("findrange", _)) => {
            let resp = send_request(proto::FindRequest {
                records: vec![proto::FindRecord {
                    id: 0,
                    filter: Some(proto::Filter {
                        location: Some(proto::filter::Location::Range(proto::Range {
                            lower: 0x10000,
                            upper: 0x100000,
                        })),
                    }),
                }],
            })?;

            let total_allocations = resp
                .allocations
                .iter()
                .fold(0, |acc, x| acc + x.allocations.len());
            println!("Found {} allocations.", total_allocations);
        }
        Some((cmd, _)) => panic!("Unhandled command: {}", cmd),
        None => panic!("How to handle None?"), // TODO: handle
    }

    Ok(())
}

fn main() {
    run(cli().get_matches()).unwrap();
}
