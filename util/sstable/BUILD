package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")

proto_library(
    name = "sstable_proto",
    srcs = ["sstable.proto"],
)

rust_proto_library(
    name = "sstable_proto_rust",
    deps = [":sstable_proto"],
)

rust_library(
    name = "sstable",
    srcs = [
        "index.rs",
        "sstable.rs",
    ],
    edition = "2018",
    deps = [
        ":sstable_proto_rust",
        "//third_party:byteorder",
        "//third_party:mmap",
        "//util/itertools",
        "//util/primitive",
        "//util/shard_lib",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "sstable_test",
    crate = ":sstable",
)
