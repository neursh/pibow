import sys
import os
import base64

def main():
    secret_key = os.urandom(32)
    ssid = sys.argv[1]
    password = sys.argv[2]

    # Apply custom values.
    lines = []
    with open("src/consts.rs", "r+") as reader:
        lines = reader.readlines()
        for index, line in enumerate(lines):
            if line.startswith("pub const WIFI_NETWORK"):
                lines[index] = f"pub const WIFI_NETWORK: &str = \"{ssid}\";\n"
            if line.startswith("pub const WIFI_PASSWORD"):
                lines[index] = f"pub const WIFI_PASSWORD: &str = \"{password}\";\n"
            if line.startswith("pub const SECRET_HASH_KEY"):
                lines[index] = f"pub const SECRET_HASH_KEY: &[u8; 32] = &[{", ".join([str(num) for num in list(secret_key)])}];\n"

    with open("src/consts.rs", "w") as writer:
        writer.writelines(lines)

    # Compile
    os.system("cargo build --release")
    try:
        os.mkdir("bin")
    except:  # noqa: E722
        pass
    os.chdir("target/thumbv6m-none-eabi/release")
    os.system("elf2uf2-rs pibow-node ../../../bin/pibow-node.uf2")
    os.remove("pibow-node")
    os.chdir("../../../")

    # Revert changes.
    for index, line in enumerate(lines):
        if line.startswith("pub const WIFI_NETWORK"):
            lines[index] = "pub const WIFI_NETWORK: &str = \"ssid\";\n"
        if line.startswith("pub const WIFI_PASSWORD"):
            lines[index] = "pub const WIFI_PASSWORD: &str = \"password\";\n"
        if line.startswith("pub const SECRET_HASH_KEY"):
            lines[index] = "pub const SECRET_HASH_KEY: &[u8; 32] = &[0_u8; 32];\n"

    with open("src/consts.rs", "w") as writer:
        writer.writelines(lines)

    print(f"Secret key (base64): {base64.b64encode(secret_key).decode("ascii")}")

if __name__ == '__main__':
    main()