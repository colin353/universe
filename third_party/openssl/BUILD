package(default_visibility = ["//visibility:public"])

licenses(["notice"])

# Open ssl's 'generated' headers
gen_hdrs = [
    "include/openssl/aes.h",
    "include/openssl/asn1.h",
    "include/openssl/asn1t.h",
    "include/openssl/asn1_mac.h",
    #    "include/openssl/async.h",
    "include/openssl/bio.h",
    "include/openssl/blowfish.h",
    "include/openssl/bn.h",
    "include/openssl/buffer.h",
    #    "include/openssl/camellia.h",
    "include/openssl/cast.h",
    "include/openssl/cmac.h",
    "include/openssl/cms.h",
    "include/openssl/comp.h",
    "include/openssl/conf_api.h",
    "include/openssl/conf.h",
    "include/openssl/crypto.h",
    "include/openssl/des.h",
    "include/openssl/des_old.h",
    "include/openssl/dh.h",
    "include/openssl/dsa.h",
    "include/openssl/dso.h",
    "include/openssl/dtls1.h",
    "include/openssl/ebcdic.h",
    "include/openssl/ecdh.h",
    "include/openssl/ecdsa.h",
    "include/openssl/ec.h",
    "include/openssl/engine.h",
    "include/openssl/e_os2.h",
    "include/openssl/err.h",
    "include/openssl/evp.h",
    "include/openssl/hmac.h",
    "include/openssl/idea.h",
    #    "include/openssl/krb_asn.h",
    "include/openssl/kssl.h",
    #    "include/openssl/jpake.h",
    #    "include/openssl/kdf.h",
    "include/openssl/krb5_asn.h",
    "include/openssl/lhash.h",
    #    "include/openssl/md2.h",
    "include/openssl/md4.h",
    "include/openssl/md5.h",
    "include/openssl/mdc2.h",
    "include/openssl/modes.h",
    "include/openssl/objects.h",
    "include/openssl/obj_mac.h",
    "include/openssl/ocsp.h",
    "include/openssl/opensslconf.h",
    "include/openssl/opensslv.h",
    "include/openssl/ossl_typ.h",
    "include/openssl/pem2.h",
    "include/openssl/pem.h",
    "include/openssl/pkcs12.h",
    "include/openssl/pkcs7.h",
    "include/openssl/pqueue.h",
    "include/openssl/rand.h",
    #    "include/openssl/rc2.h",
    #    "include/openssl/rc4.h",
    #    "include/openssl/rc5.h",
    "include/openssl/ripemd.h",
    "include/openssl/rsa.h",
    "include/openssl/safestack.h",
    "include/openssl/seed.h",
    "include/openssl/sha.h",
    "include/openssl/srp.h",
    "include/openssl/srtp.h",
    "include/openssl/ssl23.h",
    "include/openssl/ssl2.h",
    "include/openssl/ssl3.h",
    "include/openssl/ssl.h",
    "include/openssl/stack.h",
    #    "include/openssl/store.h",
    "include/openssl/symhacks.h",
    "include/openssl/tls1.h",
    "include/openssl/ts.h",
    "include/openssl/txt_db.h",
    "include/openssl/ui_compat.h",
    "include/openssl/ui.h",
    "include/openssl/whrlpool.h",
    "include/openssl/x509.h",
    "include/openssl/x509v3.h",
    "include/openssl/x509_vfy.h",
]

native.filegroup(
    name = "gen-hdrs",
    srcs = gen_hdrs,
)

# This is also conveniently used to get the directory of the
# openssl sources via bash's dirname program. Therefore, only
# allow this to have a single entry
native.filegroup(
    name = "openssl_configure_script",
    srcs = ["openssl/Configure"],
)

stat_libs = [
    "libssl.a",
    "libcrypto.a",
]

native.filegroup(
    name = "openssl-exe",
    srcs = [
        "openssl",
    ],
)

native.filegroup(
    name = "outs",
    srcs = stat_libs,
)

native.cc_library(
    name = "crypto",
    srcs = [":libcrypto.a"],
    includes = ["./openssl/include"],
    visibility = ["//visibility:public"],
)

native.cc_library(
    name = "ssl",
    srcs = [":libssl.a"],
    includes = ["./openssl/include"],
    visibility = ["//visibility:public"],
)

genrule(
    name = "build_w_openssl_buildsystem",
    srcs = [
        ":openssl_configure_script",
    ],
    outs = stat_libs + gen_hdrs,
    cmd = """
        # Get the SSL root
        OPENSSL_ROOT=$$(dirname "$(location openssl_configure_script)")
            
        # For now, just build a generic 64-bit linux version.  
        # FIXME Add other configs later
        OPENSSL_CFG_OPTS="linux-x86_64"
        
        OPENSSL_CFG_OPTS+=" no-rc2"
        OPENSSL_CFG_OPTS+=" no-rc4"
        OPENSSL_CFG_OPTS+=" no-rc5"
        OPENSSL_CFG_OPTS+=" no-camellia"
        OPENSSL_CFG_OPTS+=" no-capienga"
        OPENSSL_CFG_OPTS+=" no-inline-asm"
        
        # Unfortunately, I openssl doesn't allow out of tree builds (AFAIK)
        # cd into the sources for the next part
	    pushd $${OPENSSL_ROOT}
		./Configure $${OPENSSL_CFG_OPTS}
        popd
        CC=$(CC) make -C $${OPENSSL_ROOT} depend -j1
        CC=$(CC) AR=$(AR) make -C $${OPENSSL_ROOT} -j1
        #mkdir -p $(@D)/openssl
		
        cp $${OPENSSL_ROOT}/libcrypto.a $(location libcrypto.a)
        cp $${OPENSSL_ROOT}/libssl.a $(location libssl.a)
        cp -Lr $${OPENSSL_ROOT}/include/openssl/*.h $(@D)/include/openssl/. 
    """,
    local = 1,
)
