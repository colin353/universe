openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
openssl pkcs12 -export -out cert.p12 -inkey key.pem -in cert.pem -descert
openssl x509 -outform der -in cert.pem -out cert.der
