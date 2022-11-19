load("//util/bus:bus.bzl", "rust_bus_library")
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_bus_library(
    name = "schema",
    srcs = ["schema.bus"],
)

rust_binary(
    name = "example",
    srcs = ["example.rs"],
    deps = [
        ":schema",
        "//util/bus",
        "//util/flags",
    ],
)

rust_bus_library(
    name = "fortune_bus",
    srcs = ["fortune.bus"],
)

rust_binary(
    name = "fortune_server",
    srcs = ["fortune_server.rs"],
    deps = [
        ":fortune_bus",
        "//util/bus",
        "//util/bus:bus_rpc",
        "//third_party:tokio",
    ],
)

rust_binary(
    name = "fortune_client",
    srcs = ["fortune_client.rs"],
    deps = [
        "//util/flags",
        ":fortune_bus",
        "//util/bus",
        "//util/bus:bus_rpc",
        "//third_party:tokio",
    ],
)

rust_binary(
    name = "fortune_async_client",
    srcs = ["fortune_async_client.rs"],
    deps = [
        "//util/flags",
        ":fortune_bus",
        "//util/bus",
        "//util/bus:bus_rpc",
        "//third_party:tokio",
    ],
)
