package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "internals",
    srcs = ["internals.bus"],
)

rust_bus_library(
    name = "service",
    srcs = ["service.bus"],
)

rust_binary(
    name = "server",
    srcs = [
        "server.rs",
    ],
    deps = [
        ":managed_largetable",
        ":service", 
        "//util/bus",
        "//util/bus:bus_rpc",
        "//util/flags",
        "//third_party:tokio",
    ]
)

rust_library(
    name = "managed_largetable",
    srcs = [
        "managed.rs",
    ],
    deps = [
        ":service", 
        ":largetable",
        "//util/bus",
        "//util/flags",
    ]
)


rust_binary(
    name = "ltui",
    srcs = [
        "ltui.rs",
    ],
    deps = [
        "//util/bus",
        "//util/bus:bus_rpc",
        "//util/flags",
        ":service",
    ],
)

rust_library(
    name = "largetable",
    srcs = [
        "lib.rs",
        "dtable.rs",
        "mtable.rs",
    ],
    deps = [
        ":internals",
        "//util/bus",
        "//util/itertools",
        "//util/recordio/v2:recordio",
        "//util/sstable/v3:sstable",
    ],
)

rust_test(
    name = "largetable_test",
    crate = ":largetable"
)
