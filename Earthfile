VERSION 0.6

build-all-platforms:
  BUILD --platform=linux/amd64 --platform=linux/arm/v7 +build

chef:
  FROM ghcr.io/sksat/cargo-chef-docker:1.64.0-slim-bullseye
  WORKDIR /build

planner:
  FROM +chef
  COPY --dir src Cargo.lock Cargo.toml .
  RUN cargo chef prepare  --recipe-path recipe.json
  SAVE ARTIFACT recipe.json

# Using cutoff-optimization to ensure cache hit (see examples/cutoff-optimization)
build-cache:
  FROM +chef
  COPY +planner/recipe.json ./
  # install build deps
  RUN apt-get update && apt-get install -y libdbus-1-dev pkg-config
  RUN cargo chef cook --release
  SAVE ARTIFACT target
  SAVE ARTIFACT $CARGO_HOME cargo_home

build:
  FROM rust:1.64.0
  COPY --dir src Cargo.lock Cargo.toml .
  COPY +build-cache/cargo_home $CARGO_HOME
  COPY +build-cache/target target
  RUN apt-get update && apt-get install -y libdbus-1-dev bluez
  RUN cargo build --release
  SAVE ARTIFACT target/release/btwattch2-collector btwattch2-collector

docker:
  FROM debian:bullseye-slim
  WORKDIR /app
  ARG tag='latest'
  ARG registry=''
  RUN apt-get update && apt-get install -y libdbus-1-dev bluez
  COPY +build/btwattch2-collector btwattch2-collector
  ENTRYPOINT ["./app/btwattch2-collector"]
  SAVE IMAGE $registry''sksat/btwattch2-collector
