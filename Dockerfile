FROM gcr.io/distroless/cc
LABEL maintainer "sksat <sksat@sksat.net>"

FROM ghcr.io/sksat/cargo-chef-docker:1.63.0-slim-bullseye as chef
WORKDIR /build

FROM chef as planner
COPY Cargo.toml .
COPY Cargo.lock .
COPY src .
RUN cargo chef prepare  --recipe-path recipe.json

# build
FROM chef as builder
RUN apt-get update && apt-get install -y libdbus-1-dev pkg-config
COPY --from=planner /build/recipe.json recipe.json
COPY Cargo.toml .
COPY Cargo.lock .
COPY src/ src/
# build deps(cached)
RUN cargo chef cook --release --recipe-path recipe.json
# build bin
RUN cargo build --release

#FROM gcr.io/distroless/cc
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y libdbus-1-dev bluez
WORKDIR /app
COPY --from=builder /build/target/release/btwattch2-collector /app/
CMD ["/app/btwattch2-collector"]
