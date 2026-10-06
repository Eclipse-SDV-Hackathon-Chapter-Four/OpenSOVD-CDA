# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Loads AZ3166.pdx back with odxtools, lists the services/DIDs of every
# layer and checks a set of encode/decode round trips against the byte
# sequences in docs/diagnostics.md. Exits non-zero on any mismatch.

import os
import sys

import odxtools

SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))

failures = 0


def check(label: str, ok: bool, detail: str = ""):
    global failures
    print(f"  [{'ok' if ok else 'FAIL'}] {label}{(': ' + detail) if detail else ''}")
    if not ok:
        failures += 1


def service_line(svc) -> str:
    rq = svc.request
    try:
        raw = rq.encode() if rq is not None else b""
        rq_hex = raw.hex(" ").upper()
    except Exception:
        # Requests with value parameters need input; show the constant prefix.
        prefix = []
        for p in rq.parameters:
            if hasattr(p, "coded_value_raw"):
                n = p.get_static_bit_length() // 8
                prefix.append(int(p.coded_value_raw).to_bytes(n, "big").hex(" ").upper())
            else:
                prefix.append(f"<{p.short_name}>")
        rq_hex = " ".join(prefix)
    pre = [s.short_name for s in svc.pre_condition_states]
    return f"{svc.short_name:<44} {rq_hex:<28} {('pre=' + ','.join(pre)) if pre else ''}"


def main(pdx: str):
    db = odxtools.load_pdx_file(pdx)
    print(f"Loaded {pdx}")
    for layer in db.diag_layers:
        print(f"\n== {layer.variant_type.value} {layer.short_name} ==")
        if hasattr(layer, "ecu_variant_patterns"):
            for pat in layer.ecu_variant_patterns:
                for mp in pat.matching_parameters:
                    print(
                        f"   variant pattern: {mp.diag_comm_snref}.{mp.out_param_if_snref}"
                        f" == {int(mp.expected_value):06X}"
                    )
        for svc in sorted(layer.services, key=lambda s: s.short_name):
            print("   " + service_line(svc))

    base = db.diag_layers["AZ3166"]
    app = db.diag_layers["AZ3166_App"]
    boot = db.diag_layers["AZ3166_Boot"]

    print("\n== layer inheritance ==")

    def names(layer):
        return {s.short_name for s in layer.services}

    check("Boot has no App-only DID", "AmbientTemperature_Read" not in names(boot))
    check("App has no Boot-only service", "RequestDownload" not in names(app))
    check("App has no Programming_Start", "Programming_Start" not in names(app))
    check("Boot has Programming_Start", "Programming_Start" in names(boot))
    check("both inherit TesterPresent", all("TesterPresent" in names(v) for v in (app, boot)))
    check(
        "both inherit Identification_Read",
        all("Identification_Read" in names(v) for v in (app, boot)),
    )

    print("\n== comparams ==")
    cps = {(cp.short_name, cp.protocol_snref): cp.value for cp in base.comparam_refs}
    for name, expected in [
        ("CP_DoIPLogicalGatewayAddress", "4097"),
        ("CP_DoIPLogicalFunctionalAddress", "65535"),
        ("CP_DoIPLogicalTesterAddress", "3584"),
        ("CP_P2Max", "50000"),
        ("CP_P2Star", "5000000"),
        ("CP_P6Max", "2000000"),
        ("CP_P6Star", "8000000"),
        ("CP_RC78CompletionTimeout", "30000000"),
    ]:
        for proto in ("UDS_Ethernet_DoIP", "UDS_Ethernet_DoIP_DOBT"):
            check(f"{name} [{proto}] = {expected}", cps.get((name, proto)) == expected)
    resp_table = cps.get(("CP_UniqueRespIdTable", "UDS_Ethernet_DoIP_DOBT"))
    check("CP_UniqueRespIdTable", resp_table == ["4097", "0", "AZ3166"], str(resp_table))

    print("\n== encode / decode ==")

    def decode(layer, hexstr):
        return layer.decode(bytes.fromhex(hexstr))[0]

    m = decode(app, "62 F1 00 00 01 01")
    check("App F100 decode", m.param_dict["Identification"] == 0x000101)
    m = decode(boot, "62 F1 00 FF 00 00")
    check("Boot F100 decode", m.param_dict["Identification"] == 0xFF0000)

    m = decode(app, "62 F2 01 FF 9C")
    check(
        "F201 -10.0 degC",
        abs(m.param_dict["AmbientTemperature"] + 10.0) < 1e-9,
        str(m.param_dict["AmbientTemperature"]),
    )
    m = decode(app, "62 F2 03 27 9A")
    check(
        "F203 1013.8 hPa",
        abs(m.param_dict["AtmosphericPressure"] - 1013.8) < 1e-9,
        str(m.param_dict["AtmosphericPressure"]),
    )
    m = decode(app, "62 F2 05 00 0A FF F6 01 00")
    rate = m.param_dict["AngularRate"]
    check(
        "F205 (1.0, -1.0, 25.6) dps",
        [round(rate[k], 3) for k in "XYZ"] == [1.0, -1.0, 25.6],
        str(rate),
    )
    check("no F200", "FluxCapacitorPowerConsumption_Read" not in app.services)
    m = decode(app, "62 F2 10 02")
    check(
        "F210 B pressed",
        (m.param_dict["ButtonA"], m.param_dict["ButtonB"]) == ("released", "pressed"),
    )
    m = decode(app, "62 F1 95 30 2E 31 2E 30 20 20 20")
    check("F195 '0.1.0   '", m.param_dict["SoftwareVersion"] == "0.1.0   ")

    raw = app.services["RgbLedColor_Write"].encode_request(
        RgbLedColor={"Red": 255, "Green": 0, "Blue": 128}
    )
    check("F211 write", raw.hex() == "2ef211ff0080", raw.hex())
    raw = app.services["DisplayText_Write"].encode_request(DisplayText="Hello AZ3166    ")
    check("F212 write", raw[:3].hex() == "2ef212" and len(raw) == 19, raw.hex())

    m = decode(app, "71 01 10 01 01")
    check("SelfTest_Start -> Running", m.param_dict.get("RoutineStatus") == "Running")
    m = decode(app, "71 01 10 04 5A")
    check("VolumeDown -> 90 %", m.param_dict.get("Volume") == 90, str(m.param_dict))
    m = decode(app, "62 F2 13 64")
    check("F213 AudioVolume 100 %", m.param_dict.get("AudioVolume") == 100, str(m.param_dict))
    raw = app.services["AnnounceTemperature_Start"].encode_request()
    check("AnnounceTemperature_Start", raw.hex() == "31011002", raw.hex())
    m = decode(app, "71 03 10 02 01")
    check("AnnounceTemperature_RequestResults -> Running", m.param_dict.get("RoutineStatus") == "Running")
    m = decode(app, "71 03 10 01 02")
    check("SelfTest_RequestResults -> Completed", m.param_dict.get("RoutineStatus") == "Completed")

    m = decode(app, "59 02 FF C1 02 00 09 C1 04 00 09")
    recs = m.param_dict["DTCAndStatusRecord"]
    check("19 02 two DTC records", len(recs) == 2, str(len(recs)))
    check(
        "19 02 first DTC PressureSensorNoResponse",
        recs[0]["DTCRecord"].short_name == "PressureSensorNoResponse",
    )

    raw = boot.services["RequestDownload"].encode_request(
        MemoryAddress=0x08040000, MemorySize=0x1000
    )
    check("34 00 44 addr size", raw.hex() == "34004408040000" + "00001000", raw.hex())
    m = decode(boot, "74 20 0F FF")
    check("74 maxNumberOfBlockLength 4095", m.param_dict["MaxNumberOfBlockLength"] == 4095)
    raw = boot.services["SendKey_Level_3"].encode_request(SecurityKey=bytes.fromhex("DEADBEEF"))
    check("27 04 key", raw.hex() == "2704deadbeef", raw.hex())
    m = decode(boot, "67 03 12 34 56 78")
    check("67 03 seed", m.param_dict["SecuritySeed"] == bytes.fromhex("12345678"))

    print(f"\n{'OK' if failures == 0 else f'{failures} FAILURE(S)'}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else os.path.join(SCRIPT_DIR, "AZ3166.pdx")))
