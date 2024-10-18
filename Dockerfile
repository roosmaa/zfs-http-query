FROM rust:1.82.0-alpine as builder
RUN apk add --no-cache musl-dev
WORKDIR /usr/src/zfs-http-query
COPY . .
RUN cargo install --path .

FROM scratch
COPY --from=builder /usr/local/cargo/bin/zfs-http-query /zfs-http-query
CMD ["/zfs-http-query"]
