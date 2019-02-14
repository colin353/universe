package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")

proto_library(
    name = "weld_proto",
    srcs = ["weld.proto"],
)

rust_proto_library(
    name = "weld_proto_rust",
    deps = [":weld_proto"],
)

rust_grpc_library(
    name = "weld_grpc_rust",
    deps = [":weld_proto"],
)

rust_library(
    name = "weld",
    srcs = [
        "weld.rs",
    ],
    deps = [
        ":weld_grpc_rust",
        ":weld_proto_rust",
        "//third_party:futures",
        "//third_party:time",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_binary(
    name = "weld_util",
    srcs = ["weld_util.rs"],
    deps = [
        ":weld",
        "//util/flags",
    ],
)

rust_test(
    name = "weld_test",
    deps = [":weld"],
)

rust_library(
    name = "weld_repo",
    srcs = ["weld_repo.rs"],
    deps = [
        ":weld",
        "//largetable:largetable_client",
    ],
)

rust_binary(
    name = "weld_client",
    srcs = [
        "client_service.rs",
        "fs.rs",
        "weld_client.rs",
    ],
    deps = [
        ":weld",
        ":weld_repo",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "//third_party:fuse",
        "//third_party:libc",
        "//third_party:time",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "weld_server_lib",
    srcs = [
        "server_lib.rs",
    ],
    deps = [
        ":weld",
        ":weld_repo",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "weld_server_test",
    deps = [
        ":weld_server_lib",
    ],
)

rust_binary(
    name = "weld_server",
    srcs = ["server.rs"],
    deps = [
        ":weld",
        ":weld_repo",
        ":weld_server_lib",
        "//largetable:largetable_test",
        "//util/flags",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)
