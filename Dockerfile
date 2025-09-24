# Base stage: Install Rust and dependencies
FROM ubuntu:24.04 AS builder

WORKDIR /usr/src/app

# Install required dependencies
RUN apt-get update && apt-get install -y \
    curl \
    clang \
    build-essential \
    git \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup install stable && rustup default stable

# Install Go
RUN apt update
RUN apt install -y golang-go

# Install SP1
RUN curl -L https://sp1.succinct.xyz | bash && \
    ~/.sp1/bin/sp1up && \
    ~/.sp1/bin/cargo-prove prove --version

# Copy the entire workspace
COPY . .

# Build the proposer binary
RUN cargo build --release --bin pylon-tg-bot --locked

###############################################################################
#                                                                             #
#                                  Runtime                                    #
#                                                                             #
###############################################################################
FROM ubuntu:24.04 as runtime

WORKDIR /app

# Install only necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    curl \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH=/root/.cargo/bin:$PATH
RUN rustup install stable && rustup default stable


# Copy the built proposer binary
COPY --from=builder /usr/src/app/target/release/pylon-tg-bot /usr/local/bin/

# Set the command
CMD ["pylon-tg-bot"]
