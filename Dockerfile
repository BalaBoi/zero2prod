FROM rust:latest

WORKDIR /app

RUN apt update && apt install lld clang -y

COPY . .

RUN cargo build --release

ENTRYPOINT [ "./target/release/zer02prod" ]