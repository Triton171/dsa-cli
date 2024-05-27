FROM rust:1.78 as build

# install https://lib.rs/crates/cargo-build-dependencies so we can cache dependencies in a seperate layer
RUN cargo install cargo-build-dependencies 

# Create a new empty shell project
RUN USER=root cargo new --bin dsa-cli
WORKDIR /dsa-cli

COPY Cargo.toml Cargo.lock ./
# Build and cache dependencies
RUN cargo build-dependencies --release

# Build application
COPY ./src ./src
COPY build.rs ./
COPY ./.git ./.git
# Extract git head if .git folder exists
RUN $([ -d /.git ] && echo $(git rev-parse --short HEAD) > HEAD && rm -r .git && mkdir .git && mv HEAD .git/HEAD) || true

RUN cargo build --release

#Final base
FROM debian:stable

#Copy executable
COPY --from=build /dsa-cli/target/release/dsa-cli .

#Change the config location
ENV DSA_CLI_CONFIG_DIR=/dsa-cli-config

#Set the startup command
CMD ["./dsa-cli", "discord"]
