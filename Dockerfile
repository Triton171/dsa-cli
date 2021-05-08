FROM rust:1.51 as build

# install https://lib.rs/crates/cargo-build-dependencies so we can cache dependencies in a seperate layer
RUN cargo install cargo-build-dependencies 

# Create a new empty shell project
RUN cd /tmp && USER=root cargo new --bin dsa-cli
WORKDIR /tmp/dsa-cli

COPY Cargo.toml Cargo.lock ./
# Build and cache dependencies
RUN cargo build-dependencies --release

# Build application
COPY src ./
RUN cargo build --release

#Final base
FROM debian:stable

#Copy executable
COPY --from=build /tmp/dsa-cli/target/release/dsa-cli .

#Change the config location
ENV DSA_CLI_CONFIG_DIR=/dsa-cli-config

#Set the startup command
CMD ["./dsa-cli", "discord"]