FROM alpine:3.10.2 as build
WORKDIR /armake2-master

COPY . .
RUN apk add --no-cache libressl-dev=2.7.5-r0
RUN apk add --no-cache cargo=1.34.2-r1
RUN cargo build --release

FROM alpine:3.10.2
RUN apk add --no-cache libressl=2.7.5-r0 && \
    apk add --no-cache libgcc=8.3.0-r0
COPY --from=build /armake2-master/target/release/armake2 /usr/bin/armake2
ENTRYPOINT [ "/usr/bin/armake2" ]
