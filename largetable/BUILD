package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library", "rust_grpc_library")

proto_library(
    name = "largetable_proto",
    srcs = ["record.proto"],
)

proto_library(
    name = "largetable_service",
    srcs = ["service.proto"],
)

rust_proto_library(
    name = "largetable_proto_rust",
    deps = [":largetable_proto"],
)

rust_grpc_library(
    name = "largetable_grpc_rust",
    deps = [":largetable_service"],
)

rust_library(
    name = "keyserializer",
    srcs = ["keyserializer.rs"],
    deps = [":largetable_proto_rust"],
)

rust_library(
    name = "largetable",
    srcs = [
        "dtable.rs",
        "largetable.rs",
        "lib.rs",
        "mtable.rs",
    ],
    deps = [
        ":keyserializer",
        ":largetable_proto_rust",
        "//third_party:futures",
        "//third_party:glob",
        "//third_party:time",
        "//util/itertools",
        "//util/recordio",
        "//util/sstable",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_library(
    name = "largetable_client",
    srcs = [
        "largetable_client.rs",
    ],
    deps = [
        ":largetable",
        ":largetable_grpc_rust",
        "//third_party:futures",
        "//third_party:tokio",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "test_largetable",
    crate = ":largetable",
)

rust_binary(
    name = "largetable_server",
    srcs = [
        "largetable_server.rs",
        "server_service.rs",
    ],
    deps = [
        ":compaction",
        ":largetable",
        ":largetable_grpc_rust",
        ":largetable_proto_rust",
        "//third_party:futures",
        "//third_party:glob",
        "//third_party:time",
        "//third_party:tokio",
        "//tools/logger:logger_client",
        "//util/flags",
        "//util/sstable",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "@rules_rust//proto/raze:tls_api",
        "@rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "batching_client",
    srcs = ["batching_client.rs"],
    deps = [
        ":largetable_client",
        "//third_party:time",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "batching_client_test",
    crate = ":batching_client",
    deps = [
        ":largetable_test",
    ],
)

rust_library(
    name = "largetable_test",
    srcs = ["largetable_test.rs"],
    deps = [
        ":largetable",
        ":largetable_client",
        ":largetable_grpc_rust",
        "//third_party:time",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_library(
    name = "compaction",
    srcs = ["compaction.rs"],
    deps = [
        ":keyserializer",
        ":largetable_grpc_rust",
        ":largetable_proto_rust",
        "//third_party:time",
        "//util/sstable",
    ],
)

rust_test(
    name = "compaction_test",
    crate = ":compaction",
)

rust_test(
    name = "largetable_test_test",
    crate = ":largetable_test",
)
