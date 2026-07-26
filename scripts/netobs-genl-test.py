#!/usr/bin/env python3
"""Minimal raw-netlink smoke test for the netobs genetlink family (Phase M).
Sends NETOBS_CMD_GET_SNAPSHOT (cmd=1) as a dump to family id 0x23 and prints
the IF_NAME/IF_INDEX/OPER_STATUS attributes decoded from real reply messages.
No external deps (no pyroute2/libnl) — plain struct-packed netlink frames.
"""
import socket
import struct

NETLINK_GENERIC = 16
NETOBS_FAMILY_ID = 0x23
NETOBS_CMD_GET_SNAPSHOT = 1
NLM_F_REQUEST = 0x1
NLM_F_DUMP = 0x300

NETOBS_A_IF_INDEX = 1
NETOBS_A_IF_NAME = 2
NETOBS_A_OPER_STATUS = 3

OPER_STATUS_NAMES = {0: "Up", 1: "Down", 2: "Testing", 3: "Unknown", 4: "Dormant", 5: "NotPresent", 6: "LowerLayerDown"}

sock = socket.socket(socket.AF_NETLINK, socket.SOCK_RAW, NETLINK_GENERIC)
sock.bind((0, 0))

# nlmsghdr(len, type, flags, seq, pid) + genlmsghdr(cmd, version, reserved)
genlhdr = struct.pack("BBH", NETOBS_CMD_GET_SNAPSHOT, 1, 0)
nlmsg_len = 16 + len(genlhdr)
nlhdr = struct.pack("IHHII", nlmsg_len, NETOBS_FAMILY_ID, NLM_F_REQUEST | NLM_F_DUMP, 1, 0)
sock.send(nlhdr + genlhdr)

count = 0
while True:
    data = sock.recv(65536)
    offset = 0
    done = False
    while offset < len(data):
        msg_len, msg_type, flags, seq, pid = struct.unpack_from("IHHII", data, offset)
        if msg_type == 3:  # NLMSG_DONE
            done = True
            break
        payload = data[offset + 16 : offset + msg_len]
        cmd, version, reserved = struct.unpack_from("BBH", payload, 0)
        attrs = payload[4:]
        aoff = 0
        parsed = {}
        while aoff + 4 <= len(attrs):
            alen, atype = struct.unpack_from("HH", attrs, aoff)
            if alen < 4:
                break
            val = attrs[aoff + 4 : aoff + alen]
            if atype == NETOBS_A_IF_INDEX:
                parsed["if_index"] = struct.unpack("I", val[:4])[0]
            elif atype == NETOBS_A_IF_NAME:
                parsed["if_name"] = val.split(b"\x00")[0].decode()
            elif atype == NETOBS_A_OPER_STATUS:
                parsed["oper_status"] = OPER_STATUS_NAMES.get(struct.unpack("I", val[:4])[0], "?")
            aoff += (alen + 3) & ~3
        if parsed:
            count += 1
            print(f"  #{count} if_index={parsed.get('if_index')} name={parsed.get('if_name')} status={parsed.get('oper_status')}")
        offset += (msg_len + 3) & ~3
    if done:
        break

print(f"Total interfaces reported by kernel module: {count}")
