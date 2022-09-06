# Minecraft server in Docker

This is a Minecraft server packed as a Docker image. This project is inspired by [Minecraft on demand](https://github.com/doctorray117/minecraft-ondemand) from doctorray117, but simplifies the process for the end user by providing a simple admin panel, that will automatically shut down the server when it's idle for too long.

## Exposed ports

This image exposes the following ports:

* `8080` - Admin panel port. You need to publish this port in order to connect to the admin panel
* `25565` - Server port. You need to publish this port in order to connect to the game server

## Admin panel

When you start admin panel, it will check if the user has accepted Minecraft EULA, create `server.properties` file with provided settings and will start Minecraft server using [Aikar's flags](https://aikar.co/2018/07/02/tuning-the-jvm-g1gc-garbage-collector-flags-for-minecraft/).

Using admin panel, you can also ban/unban, kick and whitelist players, as well as start/cancel world generation. It also shows current status of the server and number of players on the server.

Admin panel is protected with login and password of your choice, to protect from unauthorized users accessing your server.

## Backup

If you want to make a backup of your Minecraft world, you should backup `/server/world` directory, or mount it on an external volume.

## Available environment variables:

| Name | Default value | Description |
| --------- | ------------------- | ----------------- |
| ADMIN_USERNAME |   | Username to access admin panel |
| ADMIN_PASSWORD |   | Password to access admin panel |
| EULA | false | Whether the user has accepted [Minecraft End User License Agreement](https://account.mojang.com/documents/minecraft_eula). Must be set to `true` in order to start the server |
| DIFFICULTY | normal | The difficulty level of the server. Available values: `peaceful`, `easy`, `normal`, `hard` |
| HARDCORE | false | Whether the hardcore mode is off or on |
| MAX_PLAYERS | 10 | The maximum number of players that can play on the server at the same time |
| MAX_WORLD_RADIUS | 1000 | The maximum possible world size in blocks, expressed as a radius. The actual world will be two times bigger than this value |
| MOTD | Minecraft on demand | Message of the day |
| PLAYER_IDLE_TIMEOUT | 10 | If non-zero, players are kicked from the server if they are idle for more than that many minutes |
| SERVER_IDLE_TIMEOUT | 10 | Server will automatically shutdown, if there are now players for more than that many minutes |
| VIEW_DISTANCE | 10 | The amount of visible chunks in each direction |
| PVP | true | Enable PvP on the server |
