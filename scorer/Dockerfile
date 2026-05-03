FROM rust:1.92-alpine AS builder
WORKDIR /src

RUN apk add --no-cache musl-dev cmake make g++ openssl-dev openssl-libs-static pkgconfig curl-dev zlib-static

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release && rm -rf src

COPY src ./src
COPY .sqlx ./.sqlx
ENV SQLX_OFFLINE=true
RUN touch src/main.rs && cargo build --release

FROM alpine:3.21 AS runtime
RUN apk add --no-cache ca-certificates libgcc
COPY --from=builder /src/target/release/scorer /scorer
CMD ["/scorer"]
