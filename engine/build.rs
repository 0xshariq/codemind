fn main() {
    for proto in [
        "../proto/v1/common.proto",
        "../proto/v1/repository.proto",
        "../proto/v1/intelligence.proto",
        "../proto/v1/memory.proto",
        "../proto/v1/operation.proto",
    ] {
        println!("cargo:rerun-if-changed={proto}");
    }

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "../proto/v1/common.proto",
                "../proto/v1/repository.proto",
                "../proto/v1/intelligence.proto",
                "../proto/v1/memory.proto",
                "../proto/v1/operation.proto",
            ],
            &["../proto"],
        )
        .expect("protobuf contracts must compile");
}
