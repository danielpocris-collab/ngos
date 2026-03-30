#!/usr/bin/env python3
import argparse
import socket
import time


def build_reply(frame: bytes, sequence: int) -> bytes:
    if len(frame) < 14:
        return b""
    dst = frame[6:12]
    src = b"\x02\x4e\x47\x4f\x53\x01"
    ethertype = frame[12:14]
    payload = b"HOST-ACK-" + bytes([sequence & 0xFF]) + b"-NGOS"
    reply = bytearray()
    reply.extend(dst)
    reply.extend(src)
    reply.extend(ethertype)
    reply.extend(payload)
    if len(reply) < 60:
        reply.extend(b"\x00" * (60 - len(reply)))
    return bytes(reply)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bind-host", default="127.0.0.1")
    parser.add_argument("--bind-port", type=int, default=10001)
    parser.add_argument("--guest-host", default="127.0.0.1")
    parser.add_argument("--guest-port", type=int, default=10000)
    parser.add_argument("--duration", type=float, default=18.0)
    parser.add_argument("--log", default="")
    args = parser.parse_args()

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind((args.bind_host, args.bind_port))
    sock.settimeout(0.25)

    end = time.time() + args.duration
    sequence = 0
    logs = []
    while time.time() < end:
        try:
            data, addr = sock.recvfrom(4096)
        except TimeoutError:
            continue
        logs.append(f"rx {len(data)} bytes from {addr}: {data[:32].hex()}")
        reply = build_reply(data, sequence)
        sequence += 1
        if reply:
            sock.sendto(reply, (args.guest_host, args.guest_port))
            logs.append(f"tx {len(reply)} bytes to {(args.guest_host, args.guest_port)}: {reply[:32].hex()}")

    if args.log:
        with open(args.log, "w", encoding="utf-8") as handle:
            handle.write("\n".join(logs))
    else:
        for line in logs:
            print(line)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
