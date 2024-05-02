package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")
load("//util/bus:bus.bzl", "rust_bus_library")

rust_bus_library(
    name = "plume_bus",
    srcs = ["plume.bus"]
)

rust_library(
    name = "plume",
    srcs = [
        "lib.rs",
        "in_memory.rs",
        "map_fn.rs",
        "sink.rs",
        "iter.rs",
    ],
    deps = [
        ":plume_bus",
        "//third_party:lazy_static",
        "//util/itertools",
        "//util/pool",
        "//util/recordio",
        "//util/shard_lib",
        "//util/sstable",
    ],
)

rust_binary(
    name = "example",
    srcs = ["example.rs"],
    deps = [ 
        ":plume",
    ],
)
