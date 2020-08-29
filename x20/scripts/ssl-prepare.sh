#!/bin/bash

cat > $HOME/.x20/config/nginx.conf << EOM
events {
  worker_connections 4096;
}
http {
  resolver 127.0.0.11 valid=30s;
  client_max_body_size 50M;
  error_log /var/log/nginx/error.log debug;
  ssl_session_cache shared:SSL:10m;
  ssl_session_timeout  20m;
  ssl_buffer_size 4k;

  server {
      listen 443 ssl;
      server_name js.colinmerkel.xyz;
      ssl_certificate /cert/cert.pem;
      ssl_certificate_key /cert/key.pem;

      location / {
        proxy_set_header    Host \$host;
        proxy_set_header    X-Real-IP \$remote_addr;
        proxy_set_header    X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header    X-Forwarded-Proto \$scheme;

        set \$proxy http://172.18.0.1:5464;
        proxy_pass          \$proxy;
        proxy_read_timeout  90;

        proxy_redirect      \$proxy https://colinmerkel.xyz;
      }
  }
}
EOM


if test -f "$HOME/.x20/data/ssl/key.pem"; then
    exit 0
fi

mkdir ~/.x20/data/ssl
cd ~/.x20/data/ssl
openssl req -x509 -nodes -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -subj '/CN=js.colinmerkel.xyz'
