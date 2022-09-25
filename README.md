# Minecraft server in Docker

This is a Minecraft server packed as a Docker image. This project is inspired by [Minecraft on demand](https://github.com/doctorray117/minecraft-ondemand) from doctorray117, but simplifies the process for the end user by providing a simple admin panel, that will automatically shut down the server when it's idle for too long.

In theory, admin panel can be used with any Minecraft server, but it does assume, that the server has "Chunky" mod installed and `fabric-server-launch.jar` is available.

## Exposed ports

This image exposes the following ports:

* `80` - Admin panel port. You need to publish this port in order to connect to the admin panel
* `25565` - Server port. You need to publish this port in order to connect to the game server

## Admin panel

This docker container includes a simple admin panel, that allows you to ban/unban players, kick players off the server, add/remove players from the whitelist and start/cancel world pre-generation. It also shows current server status and the number of players on the server. The admin panel will also validate server settings, as well as keep track of the number of players online, and will shutdown the server if it's been idle for too long.

When starting Minecraft server, it will use [Aikar's flags](https://aikar.co/2018/07/02/tuning-the-jvm-g1gc-garbage-collector-flags-for-minecraft/).

Admin panel is protected with login and password of your choice, to protect from unauthorized users accessing your server.

## Backup

If you want to make a backup of your Minecraft world, you should backup `/data` directory, or mount it on an external volume.

## Available environment variables

| Name | Available values | Default value | Description |
| ---- | ---------------- | ------------- | ----------- |
| ADMIN_USERNAME |   |   | Username to access admin panel |
| ADMIN_PASSWORD |   |   | Password to access admin panel |
| EULA | `true`, `false` | `false` | Whether the user has accepted [Minecraft End User License Agreement](https://account.mojang.com/documents/minecraft_eula). Must be set to `true` in order to start the server |
| DIFFICULTY | `peaceful`, `easy`, `normal`, `hard` | `normal` | The difficulty level of the server |
| HARDCORE | `true`, `false` | `false` | Whether the hardcore mode is off or on |
| MAX_PLAYERS | 1-255 | 10 | The maximum number of players that can play on the server at the same time |
| MAX_WORLD_RADIUS | 1-65535 | 1000 | The maximum possible radius of the world in blocks. The actual world will be two times bigger than this value |
| MOTD |   | `Minecraft on demand` | Message of the day |
| PLAYER_IDLE_TIMEOUT | 1-255 | 10 | Players are kicked from the server if they are idle for more than that many minutes |
| SERVER_IDLE_TIMEOUT | 1-255 | 10 | Server will automatically shutdown, if there are now players for more than that many minutes |
| VIEW_DISTANCE | 1-255 | 10 | The amount of visible chunks in each direction |
| PVP | `true`, `false` | `true` | Enable PvP on the server |