#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
"""DoIP/UDS smoke test against the AZ3166 ECU (board or host_sim).

    scripts/doip-smoke.py <ecu-ip>            # App variant checks
    scripts/doip-smoke.py <ecu-ip> --to-boot  # also: App -> Boot -> App cycle
"""

import socket
import struct
import sys
import time

PORT = 13400
TESTER = 0x0E00
ECU = 0x1001


def frame(payload_type: int, payload: bytes) -> bytes:
    return struct.pack(">BBHI", 0x02, 0xFD, payload_type, len(payload)) + payload


def recv_frame(sock: socket.socket) -> tuple[int, bytes]:
    header = recv_exact(sock, 8)
    _, _, payload_type, length = struct.unpack(">BBHI", header)
    return payload_type, recv_exact(sock, length)


def recv_exact(sock: socket.socket, n: int) -> bytes:
    data = b""
    while len(data) < n:
        chunk = sock.recv(n - len(data))
        if not chunk:
            raise ConnectionError("connection closed")
        data += chunk
    return data


class Tester:
    def __init__(self, ip: str):
        self.sock = socket.create_connection((ip, PORT), timeout=5)
        self.sock.sendall(frame(0x0005, struct.pack(">HBI", TESTER, 0, 0)))
        payload_type, payload = recv_frame(self.sock)
        assert payload_type == 0x0006 and payload[4] == 0x10, f"routing activation failed: {payload.hex()}"

    def uds(self, request: bytes) -> bytes:
        self.sock.sendall(frame(0x8001, struct.pack(">HH", TESTER, ECU) + request))
        payload_type, payload = recv_frame(self.sock)
        assert payload_type == 0x8002, f"no diagnostic ACK: {payload_type:04X} {payload.hex()}"
        payload_type, payload = recv_frame(self.sock)
        assert payload_type == 0x8001, f"unexpected {payload_type:04X}"
        return payload[4:]

    def close(self):
        self.sock.close()


def check(name: str, got: bytes, expect_prefix: bytes):
    ok = got.startswith(expect_prefix)
    print(f"{'PASS' if ok else 'FAIL'} {name:32} {got.hex(' ')}")
    if not ok:
        sys.exit(1)


def identify(ip: str):
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.settimeout(3)
    s.sendto(frame(0x0001, b""), (ip, PORT))
    data, _ = s.recvfrom(256)
    vin = data[8:25].decode()
    addr = struct.unpack(">H", data[25:27])[0]
    print(f"PASS vehicle identification         VIN {vin} address 0x{addr:04X} EID {data[27:33].hex(':')}")


def app_checks(t: Tester):
    check("variant (App)", t.uds(bytes.fromhex("22F100")), bytes.fromhex("62F100000101"))
    check("tester present", t.uds(bytes.fromhex("3E00")), bytes.fromhex("7E00"))
    check("VIN", t.uds(bytes.fromhex("22F190")), bytes.fromhex("62F190"))
    check("software version", t.uds(bytes.fromhex("22F195")), bytes.fromhex("62F195"))
    for did, name in [(0xF201, "temperature"), (0xF202, "humidity"), (0xF203, "pressure"),
                      (0xF204, "acceleration"), (0xF205, "angular rate"), (0xF206, "magnetic field"),
                      (0xF210, "buttons"), (0xF220, "IP address"), (0xF221, "MAC"), (0xF230, "uptime")]:
        resp = t.uds(bytes([0x22, did >> 8, did & 0xFF]))
        check(name, resp, bytes([0x62, did >> 8, did & 0xFF]))
    check("write RGB in default -> NRC", t.uds(bytes.fromhex("2EF211FF0000")), bytes.fromhex("7F2E"))
    check("extended session", t.uds(bytes.fromhex("1003")), bytes.fromhex("5003"))
    check("write RGB", t.uds(bytes.fromhex("2EF211004080")), bytes.fromhex("6EF211"))
    check("write display text", t.uds(bytes.fromhex("2EF212") + b"Hello DoIP      "), bytes.fromhex("6EF212"))
    check("self test start", t.uds(bytes.fromhex("31011001")), bytes.fromhex("7101100101"))
    time.sleep(2.0)
    check("self test result", t.uds(bytes.fromhex("31031001")), bytes.fromhex("7103100102"))
    check("read DTCs", t.uds(bytes.fromhex("1902FF")), bytes.fromhex("5902FF"))
    check("default session", t.uds(bytes.fromhex("1001")), bytes.fromhex("5001"))


def boot_cycle(ip: str, t: Tester):
    check("hard reset -> Boot", t.uds(bytes.fromhex("1101")), bytes.fromhex("5101"))
    t.close()
    time.sleep(4)
    t = Tester(ip)
    check("variant (Boot)", t.uds(bytes.fromhex("22F100")), bytes.fromhex("62F100FF0000"))
    check("programming session", t.uds(bytes.fromhex("1002")), bytes.fromhex("5002"))
    check("download without security", t.uds(bytes.fromhex("3400440000000000001000")), bytes.fromhex("7F3433"))
    seed = t.uds(bytes.fromhex("2703"))
    check("request seed", seed, bytes.fromhex("6703"))
    key = struct.unpack(">I", seed[2:6])[0] ^ 0xDEADBEEF
    check("send key", t.uds(bytes.fromhex("2704") + struct.pack(">I", key)), bytes.fromhex("6704"))
    check("request download", t.uds(bytes.fromhex("3400440000000000001000")), bytes.fromhex("74200FFF"))
    check("transfer data", t.uds(bytes.fromhex("3601") + bytes(256)), bytes.fromhex("7601"))
    check("transfer exit", t.uds(bytes.fromhex("37")), bytes.fromhex("77"))
    check("hard reset -> App", t.uds(bytes.fromhex("1101")), bytes.fromhex("5101"))
    t.close()
    time.sleep(4)
    t = Tester(ip)
    check("variant (App again)", t.uds(bytes.fromhex("22F100")), bytes.fromhex("62F100000101"))
    t.close()


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    ip = sys.argv[1]
    identify(ip)
    t = Tester(ip)
    app_checks(t)
    if "--to-boot" in sys.argv:
        boot_cycle(ip, t)
    else:
        t.close()
    print("ALL PASSED")


if __name__ == "__main__":
    main()
