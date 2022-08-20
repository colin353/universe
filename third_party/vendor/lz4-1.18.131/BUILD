package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_library(
    name = "lz4",
    srcs = [ 
      "lib.rs",
      "liblz4.rs",
      "encoder.rs",
      "decoder.rs",
      "block/mod.rs",
      "lz4_sys.rs",
    ],
    deps = [
      ":lz4_cc"
    ]
)

cc_library(
    name = "lz4_cc",
    srcs = [ 
      "lz4.c",
      "lz4frame.c",
      "lz4hc.c",
      "xxhash.c",
    ],
    hdrs = [
      "lz4.c",
      "lz4.h",
      "lz4frame.h",
      "lz4hc.h",
      "xxhash.h",
    ],
)
