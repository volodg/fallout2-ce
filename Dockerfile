# Initial image
FROM ubuntu:22.04

# Update, install, and configure
RUN set -ex \
    && apt update \
    && DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC apt install -y cmake build-essential curl git \
    && curl https://sh.rustup.rs -sSf | bash -s -- -y \
    && apt install -y libsdl2-dev

# Setting up working dir for getting whole project
# Needed for compilation
WORKDIR /app
# Copy the fallout project
COPY . .
# Set up the working dir
WORKDIR /app
# Build
RUN cmake -B build_linux && cmake --build build_linux
