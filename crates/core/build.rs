fn main() {
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());
    let files: Vec<_> = [
        "ukey",
        "securemessage",
        "securegcm",
        "device_to_device_messages",
        "offline_wire_formats",
        "wire_format",
        "sharing_enums",
    ]
    .iter()
    .map(|s| format!("proto/{s}.proto"))
    .collect();
    for file in &files {
        println!("cargo:rerun-if-changed={file}");
    }
    prost_build::compile_protos(&files, &["proto"]).unwrap();
}
