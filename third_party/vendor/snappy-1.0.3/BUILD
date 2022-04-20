package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "snappy",
    srcs = [
        "bytes.rs",
        "compress.rs",
        "crc32.rs",
        "crc32_table.rs",
        "decompress.rs",
        "error.rs",
        "frame.rs",
        "lib.rs",
        "raw.rs",
        "read.rs",
        "tag.rs",
        "varint.rs",
        "write.rs",
    ],
    edition = "2018",
)
