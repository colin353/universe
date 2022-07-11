package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "internals",
    srcs = ["internals.bus"],
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
        "//util/recordio/v2:recordio",
        "//util/sstable/v3:sstable",
    ],
)

rust_test(
    name = "largetable_test",
    crate = ":largetable"
)
