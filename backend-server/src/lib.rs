use std::{io, iter};

use allocation_catcher_backend::{
    storage::{Address, Allocation},
    wordsize, StateRef,
};

use bytes::{Bytes, BytesMut};
use common::{proto, PacketId};
use num_enum::TryFromPrimitive;
use prost::Message;

pub mod server;

pub use server::{serve_stream, serve_tcp, RequestHandler};

pub struct SimpleServer {
    state: StateRef,
}

impl RequestHandler for SimpleServer {
    fn handle_request(&self, mut packet: Bytes) -> io::Result<Bytes> {
        let packet_id_num = packet[0];
        let packet_id = PacketId::try_from_primitive(packet_id_num)
            .map_err(|_| io::Error::from(io::ErrorKind::ConnectionReset))?;
        self.request_inner(packet_id, packet.split_off(1))
            .ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionReset))
    }
}

impl SimpleServer {
    pub const fn new(state: StateRef) -> Self {
        Self { state }
    }

    fn handle_find(&self, req: proto::FindRequest) -> Vec<proto::FoundAllocation> {
        let storage = self.state.lock_storage();

        let find_record = |record: &proto::FindRecord| {
            let allocations = if let Some(filter) = record.filter.as_ref() {
                let location = filter.location.as_ref().expect("location must be set");
                match location {
                    proto::filter::Location::Address(address) => {
                        if let Some(allocation) = storage.find(*address as Address) {
                            Box::new(iter::once(allocation))
                        } else {
                            Box::new(iter::empty()) as Box<dyn Iterator<Item = &Allocation>>
                        }
                    }
                    proto::filter::Location::Range(range) => {
                        storage.find_range(range.lower as Address, range.upper as Address)
                    }
                }
            } else {
                storage.dump()
            };

            proto::FoundAllocation {
                id: record.id,
                allocations: allocations.map(|x| x.into()).collect(),
            }
        };

        req.records.iter().map(find_record).collect()
    }

    fn request_inner(&self, packet_id: PacketId, data: Bytes) -> Option<Bytes> {
        let mut response = BytesMut::new();

        match packet_id {
            PacketId::Ping => {
                let req = proto::PingRequest::decode(data).ok()?;

                proto::PingResponse {
                    version: 1,
                    num: req.num,
                    wordsize: wordsize(),
                }
                .encode(&mut response)
                .ok()?;
            }
            PacketId::SetConfiguration => {
                let req = proto::SetConfigurationRequest::decode(data).ok()?;

                self.state.set_configuration(req.configuration?.into());

                proto::SetConfigurationResponse {}
                    .encode(&mut response)
                    .ok()?;
            }
            PacketId::GetConfiguration => {
                let _req = proto::GetConfigurationRequest::decode(data).ok()?;

                proto::GetConfigurationResponse {
                    configuration: Some(self.state.get_configuration().into()),
                }
                .encode(&mut response)
                .ok()?;
            }
            PacketId::ClearStorage => {
                let _req = proto::ClearStorageRequest::decode(data).ok()?;

                self.state.lock_storage().clear();

                proto::ClearStorageResponse {}.encode(&mut response).ok()?;
            }
            PacketId::Find => {
                let req = proto::FindRequest::decode(data).ok()?;

                proto::FindResponse {
                    allocations: self.handle_find(req),
                }
                .encode(&mut response)
                .ok()?;
            }
            PacketId::GetStatistics => {
                let _req = proto::GetStatisticsRequest::decode(data).ok()?;

                let statistics = (**self.state.lock_statistics()).clone();
                let allocated = self.state.lock_storage().count();

                proto::GetStatisticsResponse {
                    statistics: Some(proto::Statistics {
                        total_allocations: statistics.total_allocations as u64,
                        total_reallocations: statistics.total_reallocations as u64,
                        total_deallocations: statistics.total_deallocations as u64,
                        total_deallocations_non_allocated: statistics
                            .total_deallocations_non_allocated
                            as u64,
                        allocated: allocated as u64,
                    }),
                }
                .encode(&mut response)
                .ok()?;
            }
            PacketId::ResetStatistics => {
                let _req = proto::ResetStatisticsRequest::decode(data).ok()?;

                self.state.lock_statistics().reset();

                proto::ResetStatisticsResponse {}
                    .encode(&mut response)
                    .ok()?;
            }
        }

        Some(response.freeze())
    }
}
