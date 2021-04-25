FROM rust:1.51 as build

#Create a new empty shell project
RUN USER=root cargo new --bin dsa-cli
WORKDIR /dsa-cli

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

#Cache dependencies
RUN cargo build --release
RUN rm src/*.rs

#Copy source
COPY ./src ./src

#Build project
RUN rm ./target/release/deps/dsa_cli*
RUN cargo build --release

#Final base
FROM debian:stable

#Copy executable
COPY --from=build /dsa-cli/target/release/dsa-cli .

#Change the config location
ENV DSA_CLI_CONFIG_DIR=/dsa-cli-config

#Set the startup command
CMD ["./dsa-cli", "discord"]