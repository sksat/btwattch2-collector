FROM gcr.io/distroless/cc
LABEL maintainer "sksat <sksat@sksat.net>"

FROM rust:1.62.0 as chef
# depName=LukeMathWalker/cargo-chef datasource=github-releases
ARG CARGO_CHEF_VERSION="v0.1.36"
RUN cargo install --version "${CARGO_CHEF_VERSION#v}" cargo-chef
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
COPY ./bluezuser.conf /etc/dbus-1/system.d/
RUN apt-get update && apt-get install -y libdbus-1-dev bluez sudo
RUN useradd -m bluezuser && adduser bluezuser sudo
RUN passwd -d bluezuser
USER bluezuser
WORKDIR /app
COPY --from=builder /build/target/release/btwattch2-collector /app/
CMD ["/app/btwattch2-collector"]
