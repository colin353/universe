package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary")

rust_library(
    name = "tui",
    srcs = [
        "lib.rs",
    ],
    edition = "2018",
    deps = [
        "//third_party:libc",
        "//third_party/raw_tty",
    ],
)

rust_binary(
    name = "example",
    srcs = [
        "example.rs",
    ],
    edition = "2018",
    deps = [":tui"],
)

rust_library(
    name = "sel",
    srcs = ["sel.rs"],
    deps = [ 
      ":tui",
      "//third_party/raw_tty",
    ],
)

rust_library(
    name = "input",
    srcs = ["input.rs"],
    deps = [ 
      ":tui",
    ],
)

rust_library(
    name = "filter",
    srcs = ["filter.rs"],
    deps = [
      ":tui",
      ":input",
    ],
)

rust_binary(
    name = "input_example",
    srcs = ["input_example.rs"],
    deps = [ 
      ":tui",
      ":input",
      ":filter",
      "//third_party/raw_tty",
      "//util/flags",
    ],
)

rust_binary(
    name = "sel_example",
    srcs = ["sel_example.rs"],
    edition = "2018",
    deps = [ 
      ":tui", 
      ":sel",
      "//third_party/raw_tty",
    ]
 )
