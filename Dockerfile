FROM rust:latest as build

WORKDIR /work

COPY . .

RUN cargo build --release

FROM ubuntu:jammy

RUN apt update
RUN apt install -y fonts-noto
RUN apt install chromium-browser -y

WORKDIR /work

COPY --from=build /work/target/release/chrome-service /work/chrome-service
COPY static /work/static

ENTRYPOINT [ "/work/chrome-service", "https://google.com/" ]
