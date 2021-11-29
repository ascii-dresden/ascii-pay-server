fn compile_protobuf() {
    println!("cargo:rerun-if-changed=proto/authentication.proto");

    protoc_grpcio::compile_grpc_protos(&["proto/authentication.proto"], &[""], "src/grpc", None)
        .expect("Failed to compile gRPC definitions!");
}

fn main() {
    compile_protobuf();
}
