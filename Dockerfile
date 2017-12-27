FROM ekidd/rust-musl-builder:nightly as builder

VOLUME /home/rust/.rustup

COPY src/* /home/rust/src/src/
COPY Cargo.* /home/rust/src/

WORKDIR /home/rust/src
RUN rustup toolchain install nightly-2017-12-20 && rustup override set nightly-2017-12-20 && rustup target add x86_64-unknown-linux-musl && cargo build --verbose --release

FROM scratch

ENV SSL_CERT_FILE /etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR /etc/ssl/certs

ADD ca-certificates.crt /etc/ssl/certs/
COPY  --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/debitoor-extensions /
EXPOSE 8080

ADD Rocket.toml /
ADD templates /templates

CMD ["/debitoor-extensions"]