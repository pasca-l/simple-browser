FROM --platform=linux/amd64 rust:1.82-bullseye

WORKDIR /home/local/

RUN apt update -y
RUN apt install -y qemu-system
