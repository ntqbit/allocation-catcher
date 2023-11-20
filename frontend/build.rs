use prost_build::Config;

fn main() {
    Config::new()
        .compile_protos(&["../common/proto/messages.proto"], &["../common/proto"])
        .expect("Protobuf build failed");
}
