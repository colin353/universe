package(default_visibility = ["//visibility:public"])

load("//util/bus:bus.bzl", "rust_bus_library")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_bus_library(
    name = "sstable_bus",
    srcs = ["sstable.bus"]
)

rust_library(
    name = "sstable",
    srcs = ["lib.rs", "bloom_filter.rs"],
    deps = [
        ":sstable_bus",
        "//third_party:mmap",
        "//util/shard_lib",
        "//util/bus",
    ]
)

rust_test(
    name = "sstable_test",
    crate = ":sstable",
)
