package(default_visibility = ["//visibility:public"])

load("@io_bazel_rules_rust//proto:proto.bzl", "rust_proto_library")
load("@io_bazel_rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_library")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_binary")
load("@io_bazel_rules_rust//rust:rust.bzl", "rust_test")
load("@io_bazel_rules_docker//rust:image.bzl", "rust_image")
load("@io_bazel_rules_docker//docker:docker.bzl", "docker_push")

proto_library(
    name = "queue_proto",
    srcs = ["queue.proto"],
)

rust_grpc_library(
    name = "queue_grpc_rust",
    deps = ["queue_proto"],
)

rust_library(
    name = "server_lib",
    srcs = ["server_lib.rs"],
    edition = "2018",
    deps = [
        ":queue_client",
        ":queue_grpc_rust",
        "//largetable:largetable_client",
        "//largetable:largetable_test",
        "//tools/lockserv:lockserv_client",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_library(
    name = "queue_client",
    srcs = ["client.rs"],
    edition = "2018",
    deps = [
        ":queue_grpc_rust",
        "//tools/lockserv:lockserv_client",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_test(
    name = "queue_test",
    crate = ":server_lib",
    edition = "2018",
    deps = [
        "//largetable:largetable_test",
    ],
)

rust_binary(
    name = "example",
    srcs = ["example.rs"],
    edition = "2018",
    deps = [
        ":queue_client",
        "//tools/lockserv:lockserv_client",
        "@io_bazel_rules_rust//proto/raze:protobuf",
    ],
)

rust_binary(
    name = "queue",
    srcs = ["server.rs"],
    edition = "2018",
    deps = [
        ":queue_grpc_rust",
        ":server_lib",
        ":webserver",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "//tools/lockserv:lockserv_client",
        "//util/flags",
        "//util/ws",
        "@io_bazel_rules_rust//proto/raze:grpc",
        "@io_bazel_rules_rust//proto/raze:protobuf",
        "@io_bazel_rules_rust//proto/raze:tls_api_stub",
    ],
)

rust_library(
    name = "webserver",
    srcs = [
        "render.rs",
        "webserver.rs",
    ],
    data = [
        "html/detail.html",
        "html/index.html",
        "html/queue.html",
        "html/template.html",
    ],
    edition = "2018",
    deps = [
        ":queue_grpc_rust",
        ":server_lib",
        "//auth:auth_client",
        "//largetable:largetable_client",
        "//util/tmpl",
        "//util/ws",
    ],
)
