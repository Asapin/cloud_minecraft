# Build admin panel
FROM rust:1.63.0-slim-bullseye as builder
WORKDIR /admin_panel

# Copy source code
COPY admin_panel .

# Build and install admin panel into /usr/local/cargo/bin/
RUN cargo install --path .

# Start Minecraft server
FROM eclipse-temurin:19-jre-jammy

WORKDIR /server
VOLUME [ "/data" ]

# Environment variables used by this image
ENV ADMIN_USERNAME="" ADMIN_PASSWORD=""
ENV EULA="" DIFFICULTY="" HARDCORE="" MAX_PLAYERS="" MAX_WORLD_RADIUS="" MOTD="" PLAYER_IDLE_TIMEOUT="" SERVER_IDLE_TIMEOUT="" VIEW_DISTANCE="" PVP=""

# Expose admin panel and game server
EXPOSE 80/tcp
EXPOSE 25565/tcp

# Copy admin panel
COPY --from=builder --chmod=755 /usr/local/cargo/bin/admin_panel admin_panel

# Copy server files and mods
COPY server .

ENTRYPOINT [ "/server/admin_panel" ]