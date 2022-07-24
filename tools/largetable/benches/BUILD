load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_binary(
    name = "backfill",
    srcs = [
        "backfill.rs"
    ],
    deps = [
        "//tools/largetable:service",
        "//util/flags",
        "//util/bus",
        "//util/bus:bus_rpc",
        "//third_party:rand",
    ]
)

rust_binary(
    name = "reads",
    srcs = [
        "reads.rs"
    ],
    deps = [
        "//tools/largetable:service",
        "//util/flags",
        "//util/bus",
        "//util/bus:bus_rpc",
        "//third_party:rand",
    ]
)
