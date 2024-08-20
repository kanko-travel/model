FROM rust:1.78-buster

RUN apt-get update && apt-get -y install clang libclang-dev cmake ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

RUN rustup default stable && rustup update

RUN useradd -m builduser
USER builduser