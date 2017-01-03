FROM scratch

ENV SSL_CERT_FILE /etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR /etc/ssl/certs

ADD ca-certificates.crt /etc/ssl/certs/
ADD target/x86_64-unknown-linux-musl/release/debitoor-extensions /
EXPOSE 8080

ADD Rocket.toml /

CMD ["/debitoor-extensions"]