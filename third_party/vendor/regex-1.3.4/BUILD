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
    "restricted",  # "MIT OR Apache-2.0"
])

load(
    "@io_bazel_rules_rust//rust:rust.bzl",
    "rust_binary",
    "rust_library",
    "rust_test",
)

# Unsupported target "backtrack" with type "test" omitted
# Unsupported target "backtrack-bytes" with type "test" omitted
# Unsupported target "backtrack-utf8bytes" with type "test" omitted
# Unsupported target "crates-regex" with type "test" omitted
# Unsupported target "default" with type "test" omitted
# Unsupported target "default-bytes" with type "test" omitted
# Unsupported target "nfa" with type "test" omitted
# Unsupported target "nfa-bytes" with type "test" omitted
# Unsupported target "nfa-utf8bytes" with type "test" omitted

rust_library(
    name = "regex",
    srcs = glob(["**/*.rs"]),
    crate_features = [
        #"aho-corasick",
        "default",
        "memchr",
        "perf",
        #"perf-cache",
        "perf-dfa",
        "perf-inline",
        #"perf-literal",
        "std",
        "thread_local",
        "unicode",
        "unicode-age",
        "unicode-bool",
        "unicode-case",
        "unicode-gencat",
        "unicode-perl",
        "unicode-script",
        "unicode-segment",
    ],
    crate_root = "src/lib.rs",
    crate_type = "lib",
    edition = "2015",
    rustc_flags = [
        "--cap-lints=allow",
    ],
    version = "1.3.4",
    deps = [
        #"//third_party/vendor/aho-corasick-0.7.9:aho_corasick",
        #"//third_party/vendor/memchr-2.3.3:memchr",
        "//third_party/vendor/regex-syntax-0.6.16:regex_syntax",
        #"//third_party/vendor/thread_local-1.0.1:thread_local",
    ],
)

# Unsupported target "shootout-regex-dna" with type "example" omitted
# Unsupported target "shootout-regex-dna-bytes" with type "example" omitted
# Unsupported target "shootout-regex-dna-cheat" with type "example" omitted
# Unsupported target "shootout-regex-dna-replace" with type "example" omitted
# Unsupported target "shootout-regex-dna-single" with type "example" omitted
# Unsupported target "shootout-regex-dna-single-cheat" with type "example" omitted
