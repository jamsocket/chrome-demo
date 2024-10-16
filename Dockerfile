FROM rust:latest AS build

WORKDIR /work

COPY . .

RUN cargo build --release

FROM debian:latest

RUN apt-get update && apt-get install -y chromium

WORKDIR /work

RUN useradd -m chrome
USER chrome

COPY --from=build /work/target/release/chrome-service /work/chrome-service
COPY static /work/static

CMD [ "/work/chrome-service", "--initial-url", "https://google.com/" ]
