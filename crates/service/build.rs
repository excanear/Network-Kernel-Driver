fn main() {
    if std::env::var("PROTOC").is_err() {
        let protoc_path = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc binary");
        std::env::set_var("PROTOC", protoc_path);
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&["proto/interface.proto"], &["proto"])
        .expect("failed to compile interface.proto");
}
