FROM clux/muslrust:stable AS builder
# set the workdir and copy the source into it
WORKDIR /app
RUN USER=root cargo new --bin builder
WORKDIR /app/builder
COPY ./Cargo.lock .
COPY ./Cargo.toml .

# load real sources
RUN rm src/*.rs
COPY ./src ./src

RUN cargo build --release --features=telemetry --bin engula-operator

# Final image
FROM debian:buster-slim
# copy the binary into the final image
COPY --from=builder /app/builder/target/x86_64-unknown-linux-musl/release/engula-operator .

# set the binary as entrypoint
ENTRYPOINT ["/engula-operator"]
