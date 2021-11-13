FROM rust:latest as build

WORKDIR /work

COPY . .

RUN cargo build --release

FROM debian:buster-slim

RUN apt-get update; apt-get install -y curl fonts-noto
RUN curl -o chrome.deb \
    https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
RUN apt-get install -y ./chrome.deb

WORKDIR /work

COPY --from=build /work/target/release/chrome-service /work/chrome-service
COPY static /work/static

ENTRYPOINT [ "/work/chrome-service", "https://google.com/" ]
