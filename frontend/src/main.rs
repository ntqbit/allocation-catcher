mod client;
mod ipc;
mod proto;

use std::io;

use clap::{arg, error::ErrorKind, ArgMatches, Command};

use client::{Client, RequestSpec};
use ipc::IpcClient;

pub fn send_request<T: RequestSpec>(msg: T) -> io::Result<T::RESPONSE> {
    let mut client = Client::new(
        IpcClient::connect("allocation-catcher")
            .map_err(|_| io::Error::from(io::ErrorKind::ConnectionRefused))?,
    );
    let response_bytes = client.request(T::PACKET_ID, msg.encode_to_vec().into())?;
    Ok(<T::RESPONSE as prost::Message>::decode(response_bytes)?)
}

fn ping() -> io::Result<()> {
    let challenge = rand::random();
    let req = proto::PingRequest { num: challenge };
    let ping_response = send_request(req)?;
    if ping_response.num == challenge {
        println!("Ping-pong! Version: {}", ping_response.version);
    } else {
        println!("Ping failed! Wrong response challenge.");
    }
    Ok(())
}

fn clear() -> io::Result<()> {
    send_request(proto::ClearStorageRequest {})?;
    println!("Done!");
    Ok(())
}

fn setcfg() -> io::Result<()> {
    send_request(proto::SetConfigurationRequest {
        configuration: Some(proto::Configuration {
            stack_trace_offset: 0x10,
            stack_trace_size: 5,
        }),
    })?;
    println!("Done!");
    Ok(())
}

fn getcfg() -> io::Result<()> {
    let resp = send_request(proto::GetConfigurationRequest {})?;
    println!("Configuration: {:#?}", resp.configuration);
    Ok(())
}

fn dump() -> io::Result<()> {
    let resp = send_request(proto::FindRequest {
        records: vec![proto::FindRecord {
            id: 0,
            filter: None,
        }],
    })?;

    assert_eq!(resp.allocations.len(), 1);

    print_allocations(&resp.allocations.first().unwrap().allocations);

    Ok(())
}

fn print_allocation(allocation: &proto::Allocation) {
    println!(
        "Allocation: [base=0x{:X},size=0x{:X}({})]",
        allocation.base_address, allocation.size, allocation.size
    );
}

fn print_allocations(allocations: &Vec<proto::Allocation>) {
    if allocations.is_empty() {
        println!("No allocations found.");
    } else {
        for allocation in allocations {
            print_allocation(allocation);
            println!();
        }
    }
}

fn find(arg: &ArgMatches) -> io::Result<()> {
    let address = *arg.get_one::<u64>("address").unwrap();
    println!("Address: 0x{address:X}");

    let resp = send_request(proto::FindRequest {
        records: vec![proto::FindRecord {
            id: 0,
            filter: Some(proto::Filter {
                location: Some(proto::filter::Location::Address(address)),
            }),
        }],
    })?;

    assert_eq!(resp.allocations.len(), 1);

    let allocations = &resp.allocations.first().unwrap().allocations;

    if let Some(allocation) = allocations.first() {
        print_allocation(allocation);
    } else {
        println!("No allocation found.");
    }

    Ok(())
}

fn findrange(cmd: &mut Command, arg: &ArgMatches) -> anyhow::Result<()> {
    let lower = *arg.get_one::<u64>("lower").unwrap();
    let upper = *arg.get_one::<u64>("upper").unwrap();

    if lower > upper {
        return Err(cmd
            .error(
                ErrorKind::InvalidValue,
                "the lower bound must not be greater than the upper bound",
            )
            .into());
    }

    println!("Range: 0x{lower:X}-0x{upper:X}");

    let resp = send_request(proto::FindRequest {
        records: vec![proto::FindRecord {
            id: 0,
            filter: Some(proto::Filter {
                location: Some(proto::filter::Location::Range(proto::Range {
                    lower,
                    upper,
                })),
            }),
        }],
    })?;

    assert_eq!(resp.allocations.len(), 1);

    print_allocations(&resp.allocations.first().unwrap().allocations);

    Ok(())
}

fn run(mut cmd: Command) -> anyhow::Result<()> {
    match cmd.get_matches_mut().subcommand().unwrap() {
        ("ping", _) => ping()?,
        ("clear", _) => clear()?,
        ("setcfg", _) => setcfg()?,
        ("getcfg", _) => getcfg()?,
        ("dump", _) => dump()?,
        ("find", sub) => find(sub)?,
        ("findrange", sub) => findrange(&mut cmd, sub)?,
        _ => unreachable!(),
    }

    Ok(())
}

fn parse_hex_address(s: &str) -> Result<u64, clap::Error> {
    if s.starts_with("0x") {
        if let Ok(x) = u64::from_str_radix(&s[2..], 16) {
            return Ok(x);
        }
    }

    Err(clap::Error::new(clap::error::ErrorKind::ValueValidation))
}

fn cli() -> Command {
    Command::new("allocation-catcher")
        .about("Allocation catcher")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("ping").about("Ping"))
        .subcommand(Command::new("getcfg").about("Get configuration"))
        .subcommand(Command::new("setcfg").about("Set configuration"))
        .subcommand(Command::new("clear").about("Clear storage"))
        .subcommand(Command::new("dump").about("Dump storage"))
        .subcommand(
            Command::new("find")
                .about("Find allocation")
                .arg(arg!(<address> "Address to find").value_parser(parse_hex_address)),
        )
        .subcommand(
            Command::new("findrange")
                .about("Find allocations in range")
                .arg(arg!(<lower> "Lower bound").value_parser(parse_hex_address))
                .arg(arg!(<upper> "Upper bound").value_parser(parse_hex_address)),
        )
}

fn main() {
    let cmd = cli();
    let result = run(cmd);

    match result {
        Ok(_) => {}
        Err(err) => {
            if let Some(err) = err.downcast_ref::<clap::Error>() {
                err.print().unwrap();
            } else {
                println!("Error: {:#?}", err)
            }
        }
    }
}
