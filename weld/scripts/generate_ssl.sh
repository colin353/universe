set -e

echo "Generate self-signed root CA"
openssl genrsa -out root.key 2048
openssl req -new -x509 -key root.key -out root.crt -subj "/C=CA/ST=Ontario/L=Toronto/O=Weld/OU=IT Department/CN=weld.io"
openssl x509 -outform der -in root.crt -out root.der

echo "Generate server certificate"
openssl genrsa -out server.key 2048
openssl rsa -in server.key -pubout -out server.pubkey
# Generate CSR
openssl req -new -key server.key -out server.csr -subj "/C=CA/ST=Ontario/L=Toronto/O=Weld/OU=IT Department/CN=server.weld.io"
# Sign CSR with root cert
openssl x509 -req -in server.csr -CA root.crt -CAkey root.key -CAcreateserial -out server.crt
# Generate pkcs12
openssl pkcs12 -export -out server.p12 -inkey server.key -in server.crt -descert

echo "Generate client certificate"
openssl genrsa -out client.key 2048
openssl rsa -in client.key -pubout -out client.pubkey
# Generate CSR
openssl req -new -key client.key -out client.csr -subj "/C=CA/ST=Ontario/L=Toronto/O=Weld/OU=IT Department/CN=client.weld.io"
# Sign CSR with root cert
openssl x509 -req -in client.csr -CA root.crt -CAkey root.key -CAcreateserial -out client.crt
# Generate pkcs12
openssl pkcs12 -export -out client.p12 -inkey client.key -in client.crt -descert


