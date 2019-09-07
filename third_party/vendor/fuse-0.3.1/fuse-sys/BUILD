"""
cargo-raze crate build file.

DO NOT EDIT! Replaced on runs of cargo-raze
"""

package(default_visibility = [
    # Public for visibility by "@raze__crate__version//" targets.
    #
    # Prefer access through "//third_party", which limits external
    # visibility to explicit Cargo.toml dependencies.
    "//visibility:public",
])

licenses([
    "notice",  # "MIT"
])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

rust_binary(
    name = "fuse_build_script",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "build.rs",
    data = glob(["*"]),
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "0.3.1",
    visibility = ["//visibility:private"],
    deps = [
        "//third_party/vendor/pkg-config-0.3.14:pkg_config",
    ],
)

genrule(
    name = "fuse_build_script_executor",
    srcs = glob([
        "*",
        "**/*.rs",
    ]),
    outs = ["fuse_out_dir_outputs.tar.gz"],
    cmd = "mkdir -p $$(dirname $@)/fuse_out_dir_outputs/;" +
          " (export CARGO_MANIFEST_DIR=\"$$PWD/$$(dirname $(location :Cargo.toml))\";" +
          # TODO(acmcarther): This needs to be revisited as part of the cross compilation story.
          #                   See also: https://github.com/google/cargo-raze/pull/54
          " export TARGET='x86_64-unknown-linux-gnu';" +
          " export RUST_BACKTRACE=1;" +
          " export OUT_DIR=$$PWD/$$(dirname $@)/fuse_out_dir_outputs;" +
          " export BINARY_PATH=\"$$PWD/$(location :fuse_build_script)\";" +
          " export OUT_TAR=$$PWD/$@;" +
          " cd $$(dirname $(location :Cargo.toml)) && $$BINARY_PATH && tar -czf $$OUT_TAR -C $$OUT_DIR .)",
    tags = ["no-sandbox"],
    tools = [
        ":fuse_build_script",
    ],
)

rust_library(
    name = "fuse_sys",
    srcs = glob(["**/*.rs"]),
    crate_features = [
    ],
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    out_dir_tar = ":fuse_build_script_executor",
    rustc_flags = [
        "--cap-lints=allow",
        "--cfg=feature=\"abi-7-13\"",
    ],
    version = "0.3.1",
    deps = [
        "//third_party/fuse",
        "//third_party/vendor/libc-0.2.43:libc",
        "//third_party/vendor/log-0.3.9:log",
        "//third_party/vendor/thread-scoped-1.0.2:thread_scoped",
        "//third_party/vendor/time-0.1.38:time",
    ],
)
