# Pibow

Reliably turn a computer on over local network using Raspberry Pi.

To archive this, a Raspberry Pi Pico W will be installed for each physical computer at the power pins, acting as a power button, but remotely activated via WiFi.

```
I can't get embassy to work from the online crate, so I cloned the whole thing to my workspace.
So before building, clone embassy and place it at the project's parent folder (same level as this readme)
```

# Communication

This implmentation is intended for long-term connection between a server and the Pico W (I'll call this a node).

To build the project, run `build.py`:
```
python3 build.py <ssid> <password>
```
This will replace some variables in `src/consts.rs` to match with the Wifi network provided. It will also generate a random secret key for security.

This random secret key will bed used for creating hash challenge using blake3, on both server and node to verify connection and authenticity of both ends.

When started, Pico W will open 2 ports, one for UDP endpoint, one for TCP server, and after that, it will turn into a TCP client:

- UDP: Multicast a hash challenge to the network to let the server discover the node.
- TCP Server: Let the server contact to the node to send messages reliably.
- TCP Client: Connect back to the server to receive requests.

### II. Let server knows the node

- `[UDP]` Send a 64 bytes hash challenge on the multicast channel, resend after 2 seconds when getting no response, the challenge changes when the node goes into this mode > Ex: New hash challenge when server disconnects.
- `[TCP]` Server connects to the node. Upon receiving the correct answer, stop spamming the heck out of the multicast, otherwise just disconnect, server got 2 seconds to send the answer.
- `[TCP]` Connect back to the server, wait for a challenge, send back the answer with MAC address to let the server knows which node is which.

### III. Taking server's requests

```
The node has 3 actions that will send over to server in one byte:
[0]: The machine is OFF.
[1]: The machine is ON.
[2]: Challenge. Right after this is the challenge.
```

```
The server has 2 actions:
[1]: Request a power ON.
[2]: Request a power OFF.
[3]: Request a RESET.

Always put the answer after the action: [1, ...]
```

- `[TCP]` From this point, the node automatically send a challenge, 64 bytes, with a pad action at the start, for a total of 65 bytes: `[2, ...]`.
- `[TCP]` While waiting for any action, listen for the machine's state, and report back to the server `[0]` OFF or `[1]` ON. If first connected, send it after challenge sent (by design).
- `[TCP]` Receive action flag with the answer: `[<action>, <answer>]`.
- From there, do whatever the server wants. If disconnected, the node will go back to section `II` and start all over again.
- If the server request a wrong action, like power ON when the machine is ON, nothing will happen, the node will send back the latest state of the machine to sync.

# Server implementation

Dunno, you can make it yourself, this repo only contains the pico w part of the whole thing, you can have this test python script I use to test this though:

```py
import socket
import sys
import struct
import os
from blake3 import blake3
import base64
import time

def main(argv):
    multicast_group = argv[1]
    multicast_port = int(argv[2])

    multicast = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    multicast.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 32)
    multicast.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    mreq = struct.pack("4sl", socket.inet_aton(multicast_group), socket.INADDR_ANY)
    multicast.bind(('', multicast_port))
    multicast.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)

    received = multicast.recvfrom(64)

    multicast.close()
    print(received)

    result = blake3(received[0], key=base64.b64decode("<base64key>")).digest()

    # Passive aggressive, answer the challenge, but dond care about the result.
    verify = socket.socket()
    verify.connect(received[1])
    verify.sendall(result)
    verify.close()

    # Think of this server socket is always open, and the node can connect to at any time, pls don't do like this outside of testing.
    server = socket.socket()
    server.bind(("0.0.0.0", 7325))
    server.listen(1)
    endpoint, address = server.accept()
    endpoint.sendall(os.urandom(64))
    print("MAC address:", list(endpoint.recv(38)[:6]))

    challenge = None
    while True:
        flag = endpoint.recv(1)

        if flag == bytes([2]):
            challenge = endpoint.recv(64)
            print("Setting power ON")
            endpoint.sendall(bytes([1]) + blake3(challenge, key=base64.b64decode("<base64key>")).digest())
            time.sleep(1)
        
        if flag == bytes([1]):
            print("Current: ON")
        if flag == bytes([0]):
            print("Current: OFF")

if __name__ == '__main__':
    if len(sys.argv) != 4:
        print("Usage: {0} <group address> <port> <interface ip>".format(sys.argv[0]))
        sys.exit(1)
    main(sys.argv)
```
