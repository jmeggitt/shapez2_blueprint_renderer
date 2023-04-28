# Setup container
FROM ubuntu:22.04
USER root
WORKDIR /home/app

RUN echo 'debconf debconf/frontend select Noninteractive' | debconf-set-selections

RUN apt-get update
# TODO: A bunch of these dependencies are not actually needed and we need to remove them to reduce the container size
RUN apt-get -y install curl gnupg build-essential cmake make g++ pkg-config

# Install Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy models into container
COPY OBJ/* OBJ/


# Install graphics
RUN apt-get install -y libxi-dev libglu1-mesa-dev libglew-dev xvfb xorg openbox libfontconfig1-dev


# Build dependencies before we copy the source files
COPY Cargo.toml .
COPY build.rs .
RUN mkdir /home/app/src
RUN touch /home/app/src/lib.rs
RUN cargo build
RUN rm -rf /home/app/src

# Copy source files
COPY blueprint .
COPY src src

ENV RUST_BACKTRACE=1
CMD xvfb-run -s "-ac -screen 0 7920x4320x24" cargo run -- -m ./OBJ blueprint -vvvvvv -o out.png --ssaa 4
