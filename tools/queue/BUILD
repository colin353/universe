package(default_visibility = ["//visibility:public"])

load("@rules_rust//proto:proto.bzl", "rust_proto_library")
load("@rules_rust//proto:proto.bzl", "rust_grpc_library")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "queue_bus",
    srcs = ["queue.bus"],
)

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
    deps = [
        ":queue_client",
        ":queue_bus",
        "//util/bus",
        "//tools/largetable:largetable_client",
        "//tools/lockserv:lockserv_client",
        "//third_party:futures",
        "//third_party:tokio",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_library(
    name = "queue_client",
    srcs = ["client.rs"],
    deps = [
        ":queue_bus",
        "//util/bus:bus_rpc",
        "//util/bus",
        "//tools/lockserv:lockserv_client",
        "@rules_rust//proto/raze:grpc",
        "//third_party:futures",
        "//third_party:tokio",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_binary(
    name = "example",
    srcs = ["example.rs"],
    deps = [
        ":queue_client",
        "//tools/lockserv:lockserv_client",
        "//third_party:tokio",
        "@rules_rust//proto/raze:protobuf",
    ],
)

rust_binary(
    name = "queue",
    srcs = ["server.rs"],
    edition = "2018",
    deps = [
        ":queue_bus",
        ":server_lib",
        ":webserver",
        "//auth:auth_client",
        "//tools/largetable:largetable_client",
        "//tools/lockserv:lockserv_client",
        "//third_party:tokio",
        "//third_party:futures",
        "//util/flags",
        "//util/bus:bus_rpc",
        "//util/ws",
        "@rules_rust//proto/raze:grpc",
        "@rules_rust//proto/raze:protobuf",
        "@rules_rust//proto/raze:tls_api_stub",
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
        ":queue_bus",
        ":server_lib",
        "//auth:auth_client",
        "//tools/largetable:largetable_client",
        "//util/tmpl",
        "//util/bus",
        "//util/ws",
    ],
)
