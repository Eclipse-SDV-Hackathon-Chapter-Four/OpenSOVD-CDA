# SPDX-FileCopyrightText: 2026 Copyright (c) Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Data identifier catalogue, 1:1 with docs/diagnostics.md "Data identifiers".

from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.odxtypes import DataType
from odxtools.parameters.valueparameter import ValueParameter
from odxtools.radix import Radix

from helper import find_dop, find_unit, identical_dop, ref, texttable_int_str_dop
from shared import add_service_did, ascii_dop, int_dop, linear_dop, struct_of

WRITE_SESSIONS = ["Extended"]


def add_common_dids(base: DiagLayerRaw):
    """DIDs readable in both variants (added to the base variant)."""
    dops = base.diag_data_dictionary_spec.data_object_props

    # F100 VariantIdentification, 3 bytes. Coded as uint24 so that the ECU
    # variant patterns can match on it (00 01 01 = 257, FF 00 00 = 16711680).
    # Service/parameter names are what the variant patterns reference.
    add_service_did(
        base,
        base,
        "Identification",
        "Identification",
        0xF100,
        find_dop(base, "IDENTICAL_UINT_24"),
        funct_class="Ident",
        semantic="IDENTIFICATION",
        long_name="Variant Identification",
    )

    # F186 ActiveDiagnosticSession, uint8 session ID
    add_service_did(
        base,
        base,
        "ActiveDiagnosticSessionDataIdentifier",
        "EcuSessionType",
        0xF186,
        find_dop(base, "EcuSessionType"),
        funct_class="Ident",
        semantic="IDENTIFICATION",
        long_name="Active Diagnostic Session",
    )

    # F18C EcuSerialNumber, 12 raw bytes (STM32 96-bit unique device ID)
    serial_dop = identical_dop(
        base, "EcuSerialNumber_12Byte", DataType.A_BYTEFIELD, 96, display_radix=Radix.HEX
    )
    dops.append(serial_dop)
    add_service_did(
        base,
        base,
        "EcuSerialNumber",
        "EcuSerialNumber",
        0xF18C,
        serial_dop,
        funct_class="Ident",
        semantic="IDENTIFICATION",
        long_name="ECU Serial Number",
    )

    # F195 SoftwareVersion, 8 bytes ASCII, space padded
    sw_dop = ascii_dop(base, "SoftwareVersion_8Byte", 8)
    dops.append(sw_dop)
    add_service_did(
        base,
        base,
        "SoftwareVersion",
        "SoftwareVersion",
        0xF195,
        sw_dop,
        funct_class="Ident",
        semantic="IDENTIFICATION",
        long_name="Software Version",
    )


def add_app_dids(base: DiagLayerRaw, app: DiagLayerRaw):
    """App-only DIDs (added to the App ECU variant)."""
    dops = app.diag_data_dictionary_spec.data_object_props
    structs = app.diag_data_dictionary_spec.structures

    def unit(name: str):
        return find_unit(base, name)

    def add_dop(dop):
        dops.append(dop)
        return dop

    def add_struct(struct):
        structs.append(struct)
        return struct

    # F190 VIN, 17 bytes ASCII, R; W in session 03
    vin_dop = add_dop(ascii_dop(app, "VIN_17Byte", 17))
    add_service_did(
        base,
        app,
        "VINDataIdentifier",
        "VIN",
        0xF190,
        vin_dop,
        funct_class="Ident",
        semantic="STOREDDATA",
        long_name="Vehicle Identification Number",
        write_sessions=WRITE_SESSIONS,
    )

    # F201 AmbientTemperature, sint16 * 0.1, degC
    temp_dop = add_dop(
        linear_dop(app, "Temperature_SInt16_0p1", True, 16, 0.1, unit("DegreeCelsius"))
    )
    add_service_did(
        base,
        app,
        "AmbientTemperature",
        "AmbientTemperature",
        0xF201,
        temp_dop,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Ambient Temperature",
    )

    # F202 RelativeHumidity, uint16 * 0.1, %RH
    hum_dop = add_dop(
        linear_dop(app, "Humidity_UInt16_0p1", False, 16, 0.1, unit("PerCentRelativeHumidity"))
    )
    add_service_did(
        base,
        app,
        "RelativeHumidity",
        "RelativeHumidity",
        0xF202,
        hum_dop,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Relative Humidity",
    )

    # F203 AtmosphericPressure, uint16 * 0.1, hPa
    press_dop = add_dop(linear_dop(app, "Pressure_UInt16_0p1", False, 16, 0.1, unit("HectoPascal")))
    add_service_did(
        base,
        app,
        "AtmosphericPressure",
        "AtmosphericPressure",
        0xF203,
        press_dop,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Atmospheric Pressure",
    )

    xyz = ["X", "Y", "Z"]

    # F204 Acceleration, 3 x sint16, factor 1, mg
    acc_axis = add_dop(int_dop(app, "Acceleration_SInt16", True, 16, unit("MilliG")))
    acc_struct = add_struct(struct_of(app, "AccelerationXYZ", xyz, acc_axis, 2))
    add_service_did(
        base,
        app,
        "Acceleration",
        "Acceleration",
        0xF204,
        acc_struct,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Acceleration",
    )

    # F205 AngularRate, 3 x sint16 * 0.1, dps
    gyro_axis = add_dop(
        linear_dop(app, "AngularRate_SInt16_0p1", True, 16, 0.1, unit("DegreePerSecond"))
    )
    gyro_struct = add_struct(struct_of(app, "AngularRateXYZ", xyz, gyro_axis, 2))
    add_service_did(
        base,
        app,
        "AngularRate",
        "AngularRate",
        0xF205,
        gyro_struct,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Angular Rate",
    )

    # F206 MagneticField, 3 x sint16, factor 1, mG
    mag_axis = add_dop(int_dop(app, "MagneticField_SInt16", True, 16, unit("MilliGauss")))
    mag_struct = add_struct(struct_of(app, "MagneticFieldXYZ", xyz, mag_axis, 2))
    add_service_did(
        base,
        app,
        "MagneticField",
        "MagneticField",
        0xF206,
        mag_struct,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Magnetic Field",
    )

    # F210 ButtonState, bitfield: bit 0 = A pressed, bit 1 = B pressed.
    # The bits are top-level response parameters (like the DTC status bits in
    # the reference database) rather than a STRUCTURE: the CDA mis-decodes
    # several bit-positioned parameters sharing byte 0 of a structure.
    pressed_dop = add_dop(
        texttable_int_str_dop(app, "ButtonPressed", [(0, "released"), (1, "pressed")], 1)
    )
    add_service_did(
        base,
        app,
        "ButtonState",
        "ButtonState",
        0xF210,
        None,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Button State",
        read_params=[
            ValueParameter(
                short_name=name,
                semantic="DATA",
                byte_position=3,
                bit_position=bit,
                dop_ref=ref(pressed_dop),
            )
            for bit, name in enumerate(["ButtonA", "ButtonB"])
        ],
    )

    uint8 = find_dop(base, "IDENTICAL_UINT_8")

    # F211 RgbLedColor, 3 x uint8; R; W in session 03
    rgb_struct = add_struct(struct_of(app, "RgbLedColor", ["Red", "Green", "Blue"], uint8, 1))
    add_service_did(
        base,
        app,
        "RgbLedColor",
        "RgbLedColor",
        0xF211,
        rgb_struct,
        funct_class="StoredData",
        semantic="STOREDDATA",
        long_name="RGB LED Color",
        write_sessions=WRITE_SESSIONS,
    )

    # F212 DisplayText, 16 bytes ASCII, space padded; R; W in session 03
    text_dop = add_dop(ascii_dop(app, "DisplayText_16Byte", 16))
    add_service_did(
        base,
        app,
        "DisplayText",
        "DisplayText",
        0xF212,
        text_dop,
        funct_class="StoredData",
        semantic="STOREDDATA",
        long_name="Display Text",
        write_sessions=WRITE_SESSIONS,
    )

    # F213 AudioVolume, uint8 0..100 %; R; W in session 03
    volume_dop = add_dop(int_dop(app, "Percent_UInt8", False, 8, unit("Percent")))
    add_service_did(
        base,
        app,
        "AudioVolume",
        "AudioVolume",
        0xF213,
        volume_dop,
        funct_class="StoredData",
        semantic="STOREDDATA",
        long_name="Audio Volume",
        write_sessions=WRITE_SESSIONS,
    )

    # F220 IpAddress, 4 x uint8 (dotted quad)
    ip_struct = add_struct(
        struct_of(app, "IpAddress", ["Octet1", "Octet2", "Octet3", "Octet4"], uint8, 1)
    )
    add_service_did(
        base,
        app,
        "IpAddress",
        "IpAddress",
        0xF220,
        ip_struct,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="IP Address",
    )

    # F221 MacAddress, 6 x uint8
    mac_struct = add_struct(
        struct_of(app, "MacAddress", [f"Byte{i}" for i in range(1, 7)], uint8, 1)
    )
    add_service_did(
        base,
        app,
        "MacAddress",
        "MacAddress",
        0xF221,
        mac_struct,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="MAC Address",
    )

    # F230 OperatingTime, uint32 seconds since reset
    optime_dop = add_dop(int_dop(app, "Seconds_UInt32", False, 32, unit("Second")))
    add_service_did(
        base,
        app,
        "OperatingTime",
        "OperatingTime",
        0xF230,
        optime_dop,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Operating Time",
    )

    # F240 PresenceState, uint8: 0 Clear, 1 Occupied, 2 Sensor Not Available
    presence_dop = add_dop(
        texttable_int_str_dop(
            app,
            "PresenceState",
            [(0, "Clear"), (1, "Occupied"), (2, "Sensor Not Available")],
        )
    )
    add_service_did(
        base,
        app,
        "PresenceState",
        "PresenceState",
        0xF240,
        presence_dop,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Presence State",
    )

    # F241 TemperatureAlarm: state (uint8), current temperature, window
    # minimum (baseline) and rise, each sint16 * 0.1 degC. Top-level
    # parameters like F210 (mixed types, no STRUCTURE needed).
    alarm_dop = add_dop(
        texttable_int_str_dop(app, "AlarmState", [(0, "Armed"), (1, "Triggered")])
    )
    add_service_did(
        base,
        app,
        "TemperatureAlarm",
        "TemperatureAlarm",
        0xF241,
        None,
        funct_class="CurrentData",
        semantic="CURRENTDATA",
        long_name="Temperature Alarm",
        read_params=[
            ValueParameter(
                short_name=name, semantic="DATA", byte_position=pos, dop_ref=ref(dop)
            )
            for name, pos, dop in [
                ("AlarmState", 3, alarm_dop),
                ("CurrentTemperature", 4, temp_dop),
                ("BaselineTemperature", 6, temp_dop),
                ("TemperatureRise", 8, temp_dop),
            ]
        ],
    )

    # F242 AlarmRiseThreshold, uint16 * 0.1 degC (0.1..50.0); R; W in session 03
    rise_dop = add_dop(
        linear_dop(app, "TemperatureDelta_UInt16_0p1", False, 16, 0.1, unit("DegreeCelsius"))
    )
    add_service_did(
        base,
        app,
        "AlarmRiseThreshold",
        "AlarmRiseThreshold",
        0xF242,
        rise_dop,
        funct_class="StoredData",
        semantic="STOREDDATA",
        long_name="Alarm Temperature Rise Threshold",
        write_sessions=WRITE_SESSIONS,
    )

    # F243 AlarmTimeWindow, uint16 seconds (10..3600); R; W in session 03
    window_dop = add_dop(int_dop(app, "Seconds_UInt16", False, 16, unit("Second")))
    add_service_did(
        base,
        app,
        "AlarmTimeWindow",
        "AlarmTimeWindow",
        0xF243,
        window_dop,
        funct_class="StoredData",
        semantic="STOREDDATA",
        long_name="Alarm Time Window",
        write_sessions=WRITE_SESSIONS,
    )

    # F244 AlarmFallThreshold, uint16 * 0.1 degC (0.1..50.0); R; W in session 03.
    # A triggered alarm clears when the temperature falls this much from its
    # peak (or when the cabin is clear).
    add_service_did(
        base,
        app,
        "AlarmFallThreshold",
        "AlarmFallThreshold",
        0xF244,
        rise_dop,
        funct_class="StoredData",
        semantic="STOREDDATA",
        long_name="Alarm Temperature Fall Threshold",
        write_sessions=WRITE_SESSIONS,
    )

    # F245 AlarmHotLimit, uint16 * 0.1 degC (0.0..80.0); R; W in session 03.
    # Occupied and above it for AlarmHotHoldTime in a row triggers the alarm.
    add_service_did(
        base,
        app,
        "AlarmHotLimit",
        "AlarmHotLimit",
        0xF245,
        rise_dop,
        funct_class="StoredData",
        semantic="STOREDDATA",
        long_name="Alarm Hot Temperature Limit",
        write_sessions=WRITE_SESSIONS,
    )

    # F246 AlarmHotHoldTime, uint16 seconds (1..3600); R; W in session 03
    add_service_did(
        base,
        app,
        "AlarmHotHoldTime",
        "AlarmHotHoldTime",
        0xF246,
        window_dop,
        funct_class="StoredData",
        semantic="STOREDDATA",
        long_name="Alarm Hot Hold Time",
        write_sessions=WRITE_SESSIONS,
    )
