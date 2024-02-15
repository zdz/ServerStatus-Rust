#!/bin/bash
set -ex

\rm -rf *.pem *.key *.srl *.csr ssr.ext

cat <<'EOF' >> ssr.ext
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
subjectAltName = @alt_names
[alt_names]
DNS.1 = *.ssr.rs
DNS.2 = ssr.rs
DNS.3 = *.localhost
DNS.4 = localhost
DNS.5 = ssr.server
IP.1 = 127.0.0.1
EOF


openssl req -x509 -sha256 -nodes -subj "/C=CN/CN=ServerStatusRust" -days 3650 -newkey rsa:4096 \
    -keyout ca.key -out ca.pem

openssl req -newkey rsa:4096 -nodes -subj "/C=CN/CN=ServerStatusRust" -keyout client.key -out client.csr
openssl x509 -signkey client.key -in client.csr -req -days 3650 -out client.pem
openssl x509 -req -CA ca.pem -CAkey ca.key -in client.csr -out client.pem -days 3650 \
    -CAcreateserial -extfile ssr.ext

openssl req -newkey rsa:4096 -nodes -subj "/C=CN/CN=ServerStatusRust" -keyout server.key -out server.csr
openssl x509 -signkey server.key -in server.csr -req -days 3650 -out server.pem
openssl x509 -req -CA ca.pem -CAkey ca.key -in server.csr -out server.pem -days 3650 \
    -CAcreateserial -extfile ssr.ext

\rm -rf *.srl *.csr ssr.ext
