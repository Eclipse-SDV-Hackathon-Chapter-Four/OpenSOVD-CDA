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
# Common data object properties and the ReadDataByIdentifier (0x22) /
# WriteDataByIdentifier (0x2E) / TesterPresent (0x3E) service builders.
# Modelled like the Eclipse OpenSOVD Classic Diagnostic Adapter test container
# (testcontainer/odx/shared.py, Apache-2.0).

from odxtools.compumethods.compucategory import CompuCategory
from odxtools.compumethods.compuinternaltophys import CompuInternalToPhys
from odxtools.compumethods.compurationalcoeffs import CompuRationalCoeffs
from odxtools.compumethods.compuscale import CompuScale
from odxtools.compumethods.identicalcompumethod import IdenticalCompuMethod
from odxtools.compumethods.linearcompumethod import LinearCompuMethod
from odxtools.dataobjectproperty import DataObjectProperty
from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.diagservice import DiagService
from odxtools.encoding import Encoding
from odxtools.nameditemlist import NamedItemList
from odxtools.odxtypes import DataType
from odxtools.parameters.valueparameter import ValueParameter
from odxtools.physicaltype import PhysicalType
from odxtools.request import Request
from odxtools.response import Response, ResponseType
from odxtools.standardlengthtype import StandardLengthType
from odxtools.structure import Structure
from odxtools.unit import Unit

from helper import (
    derived_id,
    did_parameter_rq,
    functional_class_ref,
    identical_dop,
    matching_request_parameter_did,
    matching_request_parameter_subfunction,
    pre_condition_state_ref,
    ref,
    sid_parameter_pr,
    sid_parameter_rq,
    subfunction_rq,
    texttable_int_str_dop,
)


def add_common_datatypes(dlr: DiagLayerRaw):
    """DOPs shared by several services; owned by the base variant."""
    dops = dlr.diag_data_dictionary_spec.data_object_props
    dops.append(identical_dop(dlr, "IDENTICAL_UINT_8", DataType.A_UINT32, 8))
    dops.append(identical_dop(dlr, "IDENTICAL_UINT_16", DataType.A_UINT32, 16))
    dops.append(identical_dop(dlr, "IDENTICAL_UINT_24", DataType.A_UINT32, 24))
    dops.append(identical_dop(dlr, "IDENTICAL_UINT_32", DataType.A_UINT32, 32))
    # 0x01 / 0x02 / 0x03 as in docs/diagnostics.md "Sessions"
    dops.append(
        texttable_int_str_dop(
            dlr,
            "EcuSessionType",
            [(0x01, "Default"), (0x02, "Programming"), (0x03, "Extended")],
        )
    )
    dops.append(texttable_int_str_dop(dlr, "TrueFalseDop", [(0, "false"), (1, "true")], 1))


# --------------------------------------------------------------------------
# DOP builders used by the DID catalogue
# --------------------------------------------------------------------------


def ascii_dop(dlr: DiagLayerRaw, short_name: str, byte_length: int) -> DataObjectProperty:
    """Fixed-length ISO-8859-1 string (space padded by the ECU)."""
    return DataObjectProperty(
        odx_id=derived_id(dlr, f"DOP.{short_name}"),
        short_name=short_name,
        compu_method=IdenticalCompuMethod(
            category=CompuCategory.IDENTICAL,
            physical_type=DataType.A_UNICODE2STRING,
            internal_type=DataType.A_UNICODE2STRING,
        ),
        diag_coded_type=StandardLengthType(
            base_data_type=DataType.A_ASCIISTRING,
            base_type_encoding=Encoding.ISO_8859_1,
            bit_length=byte_length * 8,
        ),
        physical_type=PhysicalType(base_data_type=DataType.A_UNICODE2STRING),
    )


def linear_dop(
    dlr: DiagLayerRaw,
    short_name: str,
    signed: bool,
    bit_length: int,
    factor: float,
    unit: Unit | None,
) -> DataObjectProperty:
    """physical = raw * factor, coded as a (signed) big-endian integer."""
    internal = DataType.A_INT32 if signed else DataType.A_UINT32
    return DataObjectProperty(
        odx_id=derived_id(dlr, f"DOP.{short_name}"),
        short_name=short_name,
        compu_method=LinearCompuMethod(
            category=CompuCategory.LINEAR,
            compu_internal_to_phys=CompuInternalToPhys(
                compu_scales=[
                    CompuScale(
                        compu_rational_coeffs=CompuRationalCoeffs(
                            value_type=DataType.A_FLOAT64,
                            numerators=[0, factor],
                            denominators=[1],
                        ),
                        domain_type=internal,
                        range_type=DataType.A_FLOAT64,
                    )
                ]
            ),
            physical_type=DataType.A_FLOAT64,
            internal_type=internal,
        ),
        diag_coded_type=StandardLengthType(base_data_type=internal, bit_length=bit_length),
        physical_type=PhysicalType(base_data_type=DataType.A_FLOAT64),
        unit_ref=ref(unit) if unit is not None else None,
    )


def int_dop(
    dlr: DiagLayerRaw,
    short_name: str,
    signed: bool,
    bit_length: int,
    unit: Unit | None = None,
) -> DataObjectProperty:
    """Integer with factor 1."""
    base = DataType.A_INT32 if signed else DataType.A_UINT32
    return identical_dop(dlr, short_name, base, bit_length, unit=unit)


def struct_of(
    dlr: DiagLayerRaw,
    short_name: str,
    members: list[str],
    member_dop: DataObjectProperty,
    member_byte_length: int,
) -> Structure:
    """Structure of equally typed, consecutive members (e.g. X/Y/Z)."""
    return Structure(
        odx_id=derived_id(dlr, f"STRUCT.{short_name}"),
        short_name=short_name,
        parameters=NamedItemList(
            [
                ValueParameter(
                    short_name=member,
                    semantic="DATA",
                    byte_position=i * member_byte_length,
                    dop_ref=ref(member_dop),
                )
                for i, member in enumerate(members)
            ]
        ),
    )


# --------------------------------------------------------------------------
# 0x22 / 0x2E
# --------------------------------------------------------------------------


def add_service_did(
    base: DiagLayerRaw,
    dlr: DiagLayerRaw,
    service_name: str,
    property_name: str,
    did: int,
    dop: DataObjectProperty | Structure | None,
    funct_class: str,
    semantic: str,
    long_name: str | None = None,
    write_sessions: list[str] | None = None,
    read_params: list[ValueParameter] | None = None,
):
    """<service_name>_Read (22 <did>) and, if ``write_sessions`` is given,
    <service_name>_Write (2E <did> <data>) restricted to those sessions.

    ``read_params`` replaces the single ``property_name``/``dop`` response
    parameter with explicitly positioned parameters (byte positions relative
    to the start of the positive response)."""
    request = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{service_name}_Read"),
        short_name=f"RQ_{service_name}_Read",
        parameters=NamedItemList([sid_parameter_rq(0x22), did_parameter_rq(did)]),
    )
    dlr.requests.append(request)

    response = Response(
        response_type=ResponseType.POSITIVE,
        odx_id=derived_id(dlr, f"PR.PR_{service_name}_Read"),
        short_name=f"PR_{service_name}_Read",
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x22 + 0x40),
                matching_request_parameter_did(),
                *(
                    read_params
                    or [
                        ValueParameter(
                            short_name=property_name,
                            semantic="DATA",
                            byte_position=3,
                            dop_ref=ref(dop),
                        )
                    ]
                ),
            ]
        ),
    )
    dlr.positive_responses.append(response)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{service_name}_Read"),
            short_name=f"{service_name}_Read",
            long_name=long_name,
            semantic=semantic,
            functional_class_refs=[functional_class_ref(base, funct_class)],
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
        )
    )

    if write_sessions is None:
        return

    request_write = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{service_name}_Write"),
        short_name=f"RQ_{service_name}_Write",
        parameters=NamedItemList(
            [
                sid_parameter_rq(0x2E),
                did_parameter_rq(did),
                ValueParameter(
                    short_name=property_name,
                    semantic="DATA",
                    byte_position=3,
                    dop_ref=ref(dop),
                ),
            ]
        ),
    )
    dlr.requests.append(request_write)

    response_write = Response(
        response_type=ResponseType.POSITIVE,
        odx_id=derived_id(dlr, f"PR.PR_{service_name}_Write"),
        short_name=f"PR_{service_name}_Write",
        parameters=NamedItemList([sid_parameter_pr(0x2E + 0x40), matching_request_parameter_did()]),
    )
    dlr.positive_responses.append(response_write)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{service_name}_Write"),
            short_name=f"{service_name}_Write",
            long_name=long_name,
            semantic=semantic,
            functional_class_refs=[functional_class_ref(base, funct_class)],
            request_ref=ref(request_write),
            pos_response_refs=[ref(response_write)],
            pre_condition_state_refs=[
                pre_condition_state_ref(base.state_charts["Session"].states[s])
                for s in write_sessions
            ],
        )
    )


# --------------------------------------------------------------------------
# 0x3E
# --------------------------------------------------------------------------


def add_tester_present_service(base: DiagLayerRaw, dlr: DiagLayerRaw):
    """3E 00 -> 7E 00"""
    request = Request(
        odx_id=derived_id(dlr, "RQ.RQ_TesterPresent"),
        short_name="RQ_TesterPresent",
        parameters=NamedItemList([sid_parameter_rq(0x3E), subfunction_rq(0x00, "ZeroSubFunction")]),
    )
    dlr.requests.append(request)

    response = Response(
        response_type=ResponseType.POSITIVE,
        odx_id=derived_id(dlr, "PR.PR_TesterPresent"),
        short_name="PR_TesterPresent",
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x3E + 0x40),
                matching_request_parameter_subfunction("ZeroSubFunction"),
            ]
        ),
    )
    dlr.positive_responses.append(response)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, "DC.TesterPresent"),
            short_name="TesterPresent",
            semantic="TESTERPRESENT",
            functional_class_refs=[functional_class_ref(base, "TesterPresent")],
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
        )
    )
