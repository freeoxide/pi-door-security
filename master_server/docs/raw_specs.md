Most important: goal is to create as less config as possible as this is a mvp project and it should just tune.

Postgres database with seaorm.

basic flow:
- This will run on docker.
- there will be standalone binaries which we will use to register admin users via interactive mode of docker.
- admin will create a clients entries (key (random uuidv7), and label). we will need to start the client server with this key so that it can connect to master server. and during the time of registration client will also send local (router) ip with port of both eth0 and wlan0 networks.
- admin can assign users to clients and users can manage client servers via web (dioxus) or app (flutter).


- basic roles for users (admin, user).
- keeping the up time of connected client server.
- keeping the logs of events in postgres database.
- basic username + password & otp (authenticator) login. very long expiry time for auth.
- admin can create users and assign roles.
- Rest API for viewing logs and sending commands to clients & managing configs.
