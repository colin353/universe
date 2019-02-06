package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")

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
    name = "largetable",
    srcs = [
        "dtable.rs",
        "keyserializer.rs",
        "largetable.rs",
        "lib.rs",
        "mtable.rs",
    ],
    deps = [
        ":largetable_proto_rust",
        "//third_party:futures",
        "//third_party:glob",
        "//third_party:time",
        "//util/recordio",
        "//util/sstable",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_library(
    name = "largetable_client",
    srcs = [
        "client_service.rs",
        "largetable_client.rs",
    ],
    deps = [
        ":largetable",
        ":largetable_grpc_rust",
        "//third_party:futures",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "largetable_test",
    deps = [":largetable"],
)

rust_binary(
    name = "largetable_server",
    srcs = [
        "largetable_server.rs",
        "server_service.rs",
    ],
    deps = [
        ":largetable",
        ":largetable_grpc_rust",
        "//third_party:futures",
        "//third_party:glob",
        "//third_party:time",
        "//third_party:tls_api",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)
