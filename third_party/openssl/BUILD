package(default_visibility = ["//visibility:public"])

licenses(["notice"])

stat_libs = [
    "libssl.a",
    "libcrypto.a",
]

cc_library(
    name = "crypto",
    srcs = ["libcrypto.a"],
    includes = ["./openssl/include"],
    visibility = ["//visibility:public"],
)

cc_library(
    name = "ssl",
    srcs = ["libssl.a"],
    includes = ["./openssl/include"],
    visibility = ["//visibility:public"],
)
