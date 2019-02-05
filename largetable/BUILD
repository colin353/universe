package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")

proto_library(
    name = "largetable_proto",
    srcs = [ "record.proto" ],
    )

rust_proto_library(
    name = "largetable_proto_rust",
    deps = [ ":largetable_proto" ],
    )

rust_library(
    name = "largetable",
    srcs = [
        "lib.rs",
        "mtable.rs",
        "dtable.rs",
        "largetable.rs",
        "keyserializer.rs",
        ],
    deps = [
        ":largetable_proto_rust",
        "//util/sstable",
        "//util/recordio",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "//third_party:futures",
        "//third_party:time",
        "//third_party:glob",
        ]
    )
