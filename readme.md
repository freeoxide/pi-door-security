# Pi door security

Building with rust to have as less of memory and cpu footprint as we want to run this on a raspberry pi.

Architecture:

- rust client (run on the pi)
 - responsible for triggering alarm, arm/disarm, and sending notifications.
- Axum master server (runs on a vps)
 - responsible to syncing the client server with the app
- Flutter mobile app
 - Responsible for arm/disarm.
- Dioxus web app (dashboard)
  - view logs.
  - setup configuration.
    - add users/devices.
    - timeout for re arming after disarm.


This will be a monorepo with a cargo workspace for the rust parts and separate folders for the flutter and dioxus apps.
