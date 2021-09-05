use std::io;

fn compile_protobuf() -> io::Result<()> {
    println!("cargo:rerun-if-changed=proto/authentication.proto");

    protoc_rust_grpc::Codegen::new()
        .out_dir("src/grpc")
        .input("proto/authentication.proto")
        .rust_protobuf(true)
        .run()
        .expect("protoc-rust-grpc");

    Ok(())
}

fn main() {
    compile_protobuf().unwrap();
}
