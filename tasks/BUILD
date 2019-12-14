package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")

proto_library(
    name = "tasks_proto",
    srcs = ["tasks.proto"],
)

rust_proto_library(
    name = "tasks_proto_rust",
    deps = [":tasks_proto"],
)

rust_grpc_library(
    name = "tasks_grpc_rust",
    deps = ["tasks_proto"],
)

rust_library(
    name = "server_impl",
    srcs = ["server_impl.rs"],
    deps = [
        ":task_db_client",
        ":task_lib",
        ":tasks_grpc_rust",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "//third_party:futures",
        "//third_party:tokio",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_binary(
    name = "tasks",
    srcs = ["server.rs"],
    deps = [
        ":server_impl",
        ":task_webserver",
        ":tasks_grpc_rust",
        "//largetable:largetable_test",
        "//third_party:tls_api_native_tls",
        "//util/flags",
        "//util/ws",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "server_impl_test",
    crate = ":server_impl",
    deps = [
        "//largetable:largetable_test",
    ],
)

rust_library(
    name = "task_lib",
    srcs = ["task_lib.rs"],
    deps = [
        ":tasks_grpc_rust",
        "//third_party:lazy_static",
        "//third_party:tokio",
    ],
)

rust_library(
    name = "task_db_client",
    srcs = ["client.rs"],
    deps = [
        ":tasks_grpc_rust",
        "//largetable:largetable_client",
    ],
)

rust_library(
    name = "task_webserver",
    srcs = ["webview.rs"],
    deps = [
        ":task_db_client",
        ":tasks_grpc_rust",
        "//largetable:largetable_client",
        "//util/tmpl",
        "//util/ws",
    ],
)
