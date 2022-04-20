package(default_visibility = ["//visibility:public"])

load("@rules_rust//rust:defs.bzl", "rust_library", "rust_binary", "rust_test")

rust_binary(
    name = "game",
    srcs = [
        "game.rs",
    ],
    edition = "2018",
    deps = [
        ":chess_engine",
        "//third_party:rand",
    ],
)

rust_library(
    name = "chess_engine",
    srcs = ["engine.rs"],
    edition = "2018",
    deps = [
        ":chess",
        ":parse_pgn",
    ],
)

rust_library(
    name = "chess",
    srcs = [
        "chess.rs",
    ],
    edition = "2018",
)

rust_library(
    name = "parse_pgn",
    srcs = [
        "pgn.rs",
    ],
    edition = "2018",
    deps = [":chess"],
)

rust_test(
    name = "test_parsing",
    crate = ":parse_pgn",
    edition = "2018",
)

rust_binary(
    name = "puzzles",
    srcs = ["puzzles.rs"],
    edition = "2018",
    deps = ["chess_engine"],
)

rust_test(
    name = "rules_test",
    crate = ":chess",
)

rust_test(
    name = "engine_test",
    crate = ":chess_engine",
)
