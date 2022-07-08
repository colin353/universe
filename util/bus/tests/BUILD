load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "battery",
    srcs = ["battery.bus"],
)

rust_library(
    name = "encoding",
    srcs = [
        "encoding.rs",
    ],
    deps = [
        ":battery",
        "//util/bus",
    ]
)

rust_test(
    name = "encoding_tests",
    crate = ":encoding",
)
