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
- `[TCP]` Upon receiving a server response message with the correct answer (`[69, 1, ...]`), stop spamming the heck out of the multicast, otherwise just disconnect this connection.
- `[TCP]` Send a confirmation message over to server with the Pi's unique ID: `[69, 2, ...<4 bytes>]`.

### III. Taking server's requests
- `[TCP]` The server must ask for a hash challenge (`[72, 65]`) before taking any actions, the actions will be dropped if the server didn't ask for the hash challenge.
- `[TCP]` Send over the hash challenge: `[...]`.
- `[TCP]` Receive action flag with the answer: `[<action>, ...<answer>]`.
- From there, do whatever the server wants. If disconnected, the node will go back to section `II` and start all over again.

## Server
### I. Prerequisites
- The administrator must provide full Wifi details to connect to and a secret hash key.
- When started, Pico W will open 2 ports, one for TCP server, one for UDP endpoint:
  - TCP: Let the server contact and request a jumpstart + send back success message reliably.
  - UDP: Listen for available signal from nodes in the multicast channel to connect to.

### II. Response to node's discovery message
- `[UDP]` Upon receiving a discover message from a node on the multicast channel, solve the challenge.
- `[TCP]` Connect to the TCP port on node's IP address, send back the awnswer with server response flag: `[69, 1, ...]`.
- `[TCP]` Client will either reject and disconnect the connection. Or send back a confirmation message, the connection will stay connected from here.
- Fetch the ID sent by the node, save it to let administrator name the node for easier access, also keeping a query to know which socket to send the turn on action to, keep the ID unless deleted by the admin, since the ID cannot be changed on the node.

### III. Request actions
- `[TCP]` On action given by users, ask for a hash challenge on the selected node (`[72, 65]`).
- `[TCP]` Receive the hash challenge and solve it.
- `[TCP]` Send the action with the answer: `[<action>, ...<answer>]`.
- Try to maintain the connection, the multcast listener will always running so the server will always have the chance to reconnect with the node.
