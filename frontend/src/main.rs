mod client;
mod transport;

use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    str::FromStr,
};

use anyhow::anyhow;
use clap::{arg, error::ErrorKind, value_parser, ArgMatches, Command};

use client::{proto, Client as TransportClient, RequestSpec};

pub struct Client {
    endpoint: SocketAddr,
}

impl Client {
    pub fn new(endpoint: SocketAddr) -> Self {
        Self { endpoint }
    }

    pub fn send_request<T: RequestSpec>(&self, msg: T) -> io::Result<T::RESPONSE> {
        let transport = Box::new(
            transport::connect_tcp(self.endpoint)
                .map_err(|_| io::Error::from(io::ErrorKind::ConnectionRefused))?,
        );
        let mut client = TransportClient::new(transport);
        let response_bytes = client.request(T::PACKET_ID, msg.encode_to_vec().into())?;
        Ok(<T::RESPONSE as prost::Message>::decode(response_bytes)?)
    }
}

fn ping(client: &Client) -> anyhow::Result<()> {
    let challenge = rand::random();
    let req = proto::PingRequest { num: challenge };
    let ping_response = client.send_request(req)?;
    if ping_response.num == challenge {
        println!("Ping-pong! Version: {}", ping_response.version);
    } else {
        println!("Ping failed! Wrong response challenge.");
    }
    Ok(())
}

fn clear(client: &Client) -> anyhow::Result<()> {
    client.send_request(proto::ClearStorageRequest {})?;
    println!("Done!");
    Ok(())
}

fn setcfg(_cmd: &mut Command, sub: &ArgMatches, client: &Client) -> anyhow::Result<()> {
    client.send_request(proto::SetConfigurationRequest {
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

fn getcfg(client: &Client) -> anyhow::Result<()> {
    let resp = client.send_request(proto::GetConfigurationRequest {})?;
    println!("Configuration: {:#?}", resp.configuration);
    Ok(())
}

fn dump(client: &Client) -> anyhow::Result<()> {
    let resp = client.send_request(proto::FindRequest {
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
    if let Some(stacktrace) = allocation.stack_trace.as_ref() {
        println!("Stack trace: {:X?}", stacktrace.trace);
    }
    if let Some(backtrace) = allocation.back_trace.as_ref() {
        println!("Back trace: ");
        for frame in backtrace.frames.iter() {
            println!(
                " - ip: {:X}, sp: {:X} mod: {:X}. sym: {}",
                frame.instruction_pointer,
                frame.stack_pointer,
                frame.module_base.unwrap_or_default(),
                {
                    if let Some(sym) = frame.resolved_symbols.first() {
                        format!(
                            "{} @  0x{:X}",
                            sym.name.as_ref().unwrap_or(&"".to_owned()),
                            sym.address.unwrap_or_default()
                        )
                    } else {
                        "-".to_owned()
                    }
                }
            );
        }
    }
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

fn find(arg: &ArgMatches, client: &Client) -> anyhow::Result<()> {
    let address = *arg.get_one::<u64>("address").unwrap();
    println!("Address: 0x{address:X}");

    let resp = client.send_request(proto::FindRequest {
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

fn findrange(cmd: &mut Command, arg: &ArgMatches, client: &Client) -> anyhow::Result<()> {
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

    let resp = client.send_request(proto::FindRequest {
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

fn getstat(client: &Client) -> anyhow::Result<()> {
    let resp = client.send_request(proto::GetStatisticsRequest {})?;
    if let Some(statistics) = resp.statistics.as_ref() {
        println!("Statistics: {:#?}", statistics);
        Ok(())
    } else {
        Err(anyhow!("no statistics field present"))
    }
}

fn resetstat(client: &Client) -> anyhow::Result<()> {
    client.send_request(proto::ResetStatisticsRequest {})?;
    println!("Done!");
    Ok(())
}

fn run(mut cmd: Command) -> anyhow::Result<()> {
    let matches = cmd.get_matches_mut();

    let host = matches
        .get_one::<String>("host")
        .map(String::as_str)
        .unwrap_or(&"127.0.0.1");
    let port = matches.get_one::<u16>("port").map(|&x| x).unwrap_or(9940);
    let endpoint = SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::from_str(host).map_err(|_| anyhow!("Could not parse IPv4"))?,
        port,
    ));
    let client = &Client::new(endpoint);

    match matches.subcommand().unwrap() {
        ("ping", _) => ping(client)?,
        ("clear", _) => clear(client)?,
        ("setcfg", sub) => setcfg(&mut cmd, sub, client)?,
        ("getcfg", _) => getcfg(client)?,
        ("dump", _) => dump(client)?,
        ("find", sub) => find(sub, client)?,
        ("findrange", sub) => findrange(&mut cmd, sub, client)?,
        ("getstat", _) => getstat(client)?,
        ("resetstat", _) => resetstat(client)?,
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
        .arg(arg!(--host <host> "Host"))
        .arg(arg!(--port <port> "Host"))
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
