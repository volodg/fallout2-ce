# Initial image
FROM rust:1.70 AS builder

# Update, install, and configure
RUN set -ex \
    && apt update \
    && apt install -y cmake \
    && apt install -y libsdl2-dev

# Setting up working dir for getting whole project
# Needed for compilation
WORKDIR /app
# Copy the fallout project
COPY . .
# Set up the working dir
WORKDIR /app
# Build
# RUN cmake -B build_linux && cmake --build build_linux
