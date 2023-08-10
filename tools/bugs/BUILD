package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library")
load("@rules_rust//proto:proto.bzl", "rust_grpc_library")

proto_library(
    name = "bugs_proto",
    srcs = ["bugs.proto"],
)

rust_grpc_library(
    name = "bugs_grpc_rust",
    deps = [":bugs_proto"],
)

rust_library(
    name = "bug_server_lib",
    srcs = ["server_lib.rs"],
    deps = [
        ":bugs_grpc_rust",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "bug_server_lib_test",
    crate = ":bug_server_lib",
    deps = [
        "//largetable:largetable_test",
    ],
)

rust_binary(
    name = "bug_server",
    srcs = ["server.rs"],
    deps = [
        ":bug_server_lib",
        ":bugs_grpc_rust",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "//util/flags",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "@rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "bug_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":bugs_grpc_rust",
        "//util/grpc_tls",
        "//third_party:futures",
        "@rules_rust//proto/raze:grpc",
    ],
)

rust_binary(
    name = "bug_util",
    srcs = ["util.rs"],
    data = [
        "template.txt",
    ],
    edition = "2018",
    deps = [
        ":bug_client",
        ":bugs_grpc_rust",
        "//auth:auth_client",
        "//util/cli",
        "//util/flags",
        "//util/init",
    ],
)
