mod client;
mod transport;

use std::{io, net::SocketAddr};

use anyhow::anyhow;
use clap::{arg, error::ErrorKind, value_parser, ArgMatches, Command};

use client::{proto, Client, RequestSpec};

pub fn send_request<T: RequestSpec>(msg: T) -> io::Result<T::RESPONSE> {
    let transport = Box::new(
        transport::connect_tcp(&SocketAddr::from(([127, 0, 0, 1], 9940)))
            .map_err(|_| io::Error::from(io::ErrorKind::ConnectionRefused))?,
    );
    let mut client = Client::new(transport);
    let response_bytes = client.request(T::PACKET_ID, msg.encode_to_vec().into())?;
    Ok(<T::RESPONSE as prost::Message>::decode(response_bytes)?)
}

fn ping() -> anyhow::Result<()> {
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

fn clear() -> anyhow::Result<()> {
    send_request(proto::ClearStorageRequest {})?;
    println!("Done!");
    Ok(())
}

fn setcfg(_cmd: &mut Command, sub: &ArgMatches) -> anyhow::Result<()> {
    send_request(proto::SetConfigurationRequest {
        configuration: Some(proto::Configuration {
            stack_trace_offset: *sub.get_one("stoff").unwrap(),
            stack_trace_size: *sub.get_one("stsize").unwrap(),
            backtrace_frames_skip: *sub.get_one("btskip").unwrap(),
            backtrace_frames_count: *sub.get_one("btcount").unwrap(),
            backtrace_resolve_symbols_count: *sub.get_one("btsymbols").unwrap(),
        }),
    })?;
    println!("Done!");
    Ok(())
}

fn getcfg() -> anyhow::Result<()> {
    let resp = send_request(proto::GetConfigurationRequest {})?;
    println!("Configuration: {:#?}", resp.configuration);
    Ok(())
}

fn dump() -> anyhow::Result<()> {
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
        }
    }
}

fn find(arg: &ArgMatches) -> anyhow::Result<()> {
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

fn getstat() -> anyhow::Result<()> {
    let resp = send_request(proto::GetStatisticsRequest {})?;
    if let Some(statistics) = resp.statistics.as_ref() {
        println!("Statistics: {:#?}", statistics);
        Ok(())
    } else {
        Err(anyhow!("no statistics field present"))
    }
}

fn resetstat() -> anyhow::Result<()> {
    send_request(proto::ResetStatisticsRequest {})?;
    println!("Done!");
    Ok(())
}

fn run(mut cmd: Command) -> anyhow::Result<()> {
    match cmd.get_matches_mut().subcommand().unwrap() {
        ("ping", _) => ping()?,
        ("clear", _) => clear()?,
        ("setcfg", sub) => setcfg(&mut cmd, sub)?,
        ("getcfg", _) => getcfg()?,
        ("dump", _) => dump()?,
        ("find", sub) => find(sub)?,
        ("findrange", sub) => findrange(&mut cmd, sub)?,
        ("getstat", _) => getstat()?,
        ("resetstat", _) => resetstat()?,
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
        .subcommand(
            Command::new("setcfg")
                .about("Set configuration")
                .arg(
                    arg!(--stoff <stack_trace_offset> "Stack trace offset")
                        .value_parser(value_parser!(u64))
                        .required(true),
                )
                .arg(
                    arg!(--stsize <stack_trace_size> "Stack trace size")
                        .value_parser(value_parser!(u64))
                        .required(true),
                )
                .arg(
                    arg!(--btskip <backtrace_frames_skip> "Backtrace frames skip")
                        .value_parser(value_parser!(u32))
                        .required(true),
                )
                .arg(
                    arg!(--btcount <backtrace_frames_count> "Backtrace frames count")
                        .value_parser(value_parser!(u32))
                        .required(true),
                )
                .arg(
                    arg!(--btsymbols <backtrace_resolve_symbols_count> "Backtrace resolve symbols count")
                        .value_parser(value_parser!(u32))
                        .required(true),
                ),
        )
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
        .subcommand(Command::new("getstat").about("Get statistics"))
        .subcommand(Command::new("resetstat").about("Reset statistics"))
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
