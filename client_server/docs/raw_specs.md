
1 Network redundancy
We want to ensure complete and always on internet connectivity.

We have an arm disarm system which we can trigger via 4 ways:
- bluetooth.
- local network (no internet) (maybe API call or websocket).
- cloud (Websocket).
- radio (433MHz).

Raspberry will be connected to a magnetic sensor which will trigger the arm/disarm system when the door is closed/opened.

When we disarm we wil have a timeout (configurable). after timeout it will be re armed again.

If a door opened when it's armed we will trigger alarm, on flood lights and let out master server know which we send notification to the connected phones.

client server should be fool proof if it's crashed system should restart it or what ever via systemd or something better but it should be always on.
