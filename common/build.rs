use prost_build::Config;

fn main() {
    Config::new()
        .compile_protos(&["proto/messages.proto"], &["proto"])
        .expect("Protobuf build fail");
}
