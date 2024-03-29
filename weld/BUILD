package(default_visibility = ["//visibility:public"])
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("@rules_rust//proto:proto.bzl", "rust_proto_library", "rust_grpc_library")

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
    edition = "2018",
    deps = [
        "//util/cli",
        ":weld_grpc_rust",
        ":weld_proto_rust",
        "//third_party:futures",
        "//third_party:native_tls",
        "//third_party:time",
        "//util/grpc_tls",
        "//util/tmpl:escape",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "@rules_rust//proto/raze:tls_api",
        "@rules_rust_proto__httpbis__0_9_1//:httpbis",
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
    name = "weld_test_test",
    crate = ":weld",
)

rust_library(
    name = "weld_repo",
    srcs = ["weld_repo.rs"],
    deps = [
        ":weld",
        "//largetable:batching_client",
        "//largetable:largetable_client",
        "//util/cache",
        "//weld:merge_lib",
    ],
)

rust_library(
    name = "repo_test_lib",
    srcs = ["repo_test.rs"],
    deps = [
        "//largetable:largetable_test",
        "//weld",
        "//weld:weld_repo",
        "//weld:weld_test",
    ],
)

rust_test(
    name = "weld_repo_test",
    crate = ":repo_test_lib",
)

rust_library(
    name = "client_service",
    srcs = ["client_service.rs"],
    edition = "2018",
    deps = [
        ":weld",
        ":weld_repo",
        "//largetable:largetable_client",
        "//third_party:fuse",
        "//third_party:libc",
        "//third_party:rand",
        "//third_party:time",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "@rules_rust//proto/raze:tls_api",
        "@rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_binary(
    name = "weld_client",
    srcs = [
        "fs.rs",
        "parallel_fs.rs",
        "weld_client.rs",
    ],
    deps = [
        ":client_service",
        ":weld",
        ":weld_repo",
        "//largetable:batching_client",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "//third_party:fuse",
        "//third_party:libc",
        "//third_party:rand",
        "//third_party:time",
        "//third_party:tls_api_openssl",
        "//util/flags",
        "//util/pool",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "@rules_rust//proto/raze:tls_api",
        "@rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "weld_server_lib",
    srcs = [
        "server_lib.rs",
    ],
    edition = "2018",
    deps = [
        ":weld",
        ":weld_repo",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
    ],
)
rust_test(
    name = "weld_server_test",
    crate = ":weld_server_lib",
)
rust_binary(
    name = "weld_server",
    srcs = ["server.rs"],
    edition = "2018",
    deps = [
        ":weld",
        "//auth:auth_client",
        "//util/init",
        ":weld_repo",
        ":weld_server_lib",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "//third_party:native_tls",
        "//third_party:openssl",
        "//third_party:tls_api_openssl",
        "//util/flags",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "@rules_rust//proto/raze:tls_api",
    ],
)
rust_library(
    name = "weld_test",
    srcs = ["weld_test.rs"],
    deps = [
        "//auth:auth_client",
        ":weld",
        ":weld_server_lib",
        "//largetable:largetable_test",
    ],
)
rust_binary(
    name = "merge",
    srcs = ["merge.rs"],
    deps = [
        ":merge_lib",
        "//util/flags",
    ],
)
rust_library(
    name = "merge_lib",
    srcs = ["merge_lib.rs"],
    deps = [
        "//third_party:difference",
    ],
)
rust_test(
    name = "merge_test",
    crate = ":merge_lib",
)
