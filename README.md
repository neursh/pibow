# Pibow

Reliably turn a computer on over local network using Raspberry Pi.

To archive this, a Raspberry Pi Pico W will be installed for each physical computer at the power pins, acting as a power button, but remotely activated via WiFi.

The main server will use a Pi 3/4/5, or anything with an kernel and an operating system installed to handle requests and asking Picos to jumpstart the computer.

This ensures flexibility for the main server, like install a proxy service to receive requests from outside the local network.

The node follows a simple rule: "Nothing is trusted". The server must request a new one-time hash challenge, and include the answer on the next message. If the attempt succeeded, the node will send a success flag.

```
I can't get embassy to work from the online crate, so I cloned the whole thing to my workspace.
So before building, clone embassy and place it at the project's parent folder (same level as this readme)
```

# Communication

## Pico W (node)

### I. Prerequisites

- On binary build, the administrator must provide full Wifi details to connect to and a secret hash key, this key must be shared with the server.
- When started, Pico W will open 2 ports, one for TCP server, one for UDP endpoint:
  - TCP: Let the server contact and request a jumpstart + send back success message reliably.
  - UDP: Multicast available signal to the network to let the server discover online nodes.

### II. Discover the server

- `[UDP]` Send a discover message on the multicast channel, resend after 2 seconds when getting no response, 64 bytes is a random challenge that changes when the node goes in discover mode.
- `[TCP]` Upon receiving a server response message with the correct answer, stop spamming the heck out of the multicast, otherwise just disconnect this connection.
- `[TCP]` Connect back to the server, wait for a challenge, send back answer with MAC address.

### III. Taking server's requests

The node has 3 actions, that will send over to server in one byte:

- `[0]`: The machine is OFF.
- `[1]`: The machine is ON.
- `[2]`: Challenge.

- `[TCP]` From this point, the node automatically send a challenge, 64 bytes, with a pad action at the start, for a total of 65 bytes.
- `[TCP]` Send over the hash challenge: `[2, ...]`.
- `[TCP]` While waiting for any action, listen for the machine's state, and report back to the server `[0]` OFF or `[1]` ON. If first connected, send it after challenge sent (by design).
- `[TCP]` Receive action flag with the answer: `[<action>, <answer>]`.
- From there, do whatever the server wants. If disconnected, the node will go back to section `II` and start all over again.

## Server

### I. Prerequisites

- The administrator must provide full Wifi details to connect to and a secret hash key.
- When started, Pico W will open 2 ports, one for TCP server, one for UDP endpoint:
  - TCP: Let the server contact and request a jumpstart + send back success message reliably.
  - UDP: Listen for available signal from nodes in the multicast channel to connect to.

### II. Response to node's discovery message

- `[UDP]` Upon receiving a discover message from a node on the multicast channel, solve the challenge.
- `[TCP]` Connect to the TCP port on node's IP address, send back the awnswer.
- `[TCP]` CLient will disconnect no matter what.
- `[TCP]` If the answer is correct, the node will connect back to the server, send a challenge to it.

### III. Request actions

- `[TCP]` The node will send back its MAC address and the answer, if the answer is wrong, disconnect it.
- `[TCP]` Store the challenge sent by the node for later use when an action is invoked.
- `[TCP]` Also receive reports from the node for machine's state.
- Fetch the MAC address sent by the node, save it to let administrator name the node for easier access, also keeping a query to know which socket to send the turn on action to, unless the MAC address deleted by the admin, since the MAC address cannot be changed on the node.
- Try to maintain the connection, the multcast listener will be started if disconnected so the server will always have the chance to reconnect with the node.
