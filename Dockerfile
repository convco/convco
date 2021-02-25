FROM rust:alpine as builder
RUN apk add clang musl-dev openssl-dev cmake make

COPY . /tmp
WORKDIR /tmp

RUN cargo --version
RUN cargo build --release

FROM alpine as base
COPY --from=builder /tmp/target/release/convco /usr/bin/convco

ENTRYPOINT [ "convco" ]
CMD [ "check" ]
