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
    ],
)
