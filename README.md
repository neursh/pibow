# Pibow
Reliably turn a computer on over local network using Raspberry Pi.

To archive this, a Raspberry Pi Pico W will be installed for each physical computer at the power pins, acting as a power button, but remotely activated via WiFi.

The main server will use a Pi 3/4/5, or anything with an kernel and an operating system installed to handle requests and asking Picos to jumpstart the computer.

This ensures flexibility for the main server, like install a proxy service to receive requests from outside the local network.

The node follows a simple rule: "Nothing is trusted". The server must request a new one-time hash challenge, and include the answer on the next message. If the attempt succeeded, the node will send a success flag.

# Communication
## Pico W (node)
### I. Prerequisites
- On binary build, the administrator must provide full Wifi details to connect to and a secret hash key, this key must be shared with the server.
- When started, Pico W will open 2 ports, one for TCP server, one for UDP endpoint:
  - TCP: Let the server contact and request a jumpstart + send back success message reliably.
  - UDP: Multicast available signal to the network to let the server discover online nodes.

### II. Discover the server
- `[UDP]` Send a discover message on the multicast channel, resend after 5 seconds when getting no response, with the first 2 bytes is the message type, and the rest 128 bytes is a random challenge that changes when the node goes in discover mode: `[69, 0, ...]`.
- `[TCP]` Upon receiving a server response message with the correct answer (`[69, 1, ...]`), stop spamming the heck out of the multicast, otherwise just cancel the stream.
- `[TCP]` Send a confirmation message over to server with the Pi's unique ID: `[69, 2, ...<4 bytes>]`.

### III. Taking server's requests
- `[TCP]` The server must ask for a hash challenge (`[72, 65]`) before taking any actions, the actions will be dropped if the server didn't ask for the hash challenge.
- `[TCP]` Send over the hash challenge: `[...]`.
- `[TCP]` Receive action flag with the answer: `[<action>, ...<answer>]`
- From there, do whatever the server wants. If disconnected, the node will go back to section `II` and start all over again.
