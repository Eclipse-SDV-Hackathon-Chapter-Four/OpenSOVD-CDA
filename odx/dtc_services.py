# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Fault memory: DTC catalogue, ReadDTCInformation (0x19 02) and
# ClearDiagnosticInformation (0x14). Modelled like the Eclipse OpenSOVD Classic
# Diagnostic Adapter test container (testcontainer/odx/dtc_services.py,
# Apache-2.0). App variant only, see docs/diagnostics.md "DTCs (App)".

from odxtools.compumethods.compucategory import CompuCategory
from odxtools.compumethods.compumethod import CompuMethod
from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.diagnostictroublecode import DiagnosticTroubleCode
from odxtools.diagservice import DiagService
from odxtools.dtcdop import DtcDop
from odxtools.endofpdufield import EndOfPduField
from odxtools.nameditemlist import NamedItemList
from odxtools.odxtypes import DataType
from odxtools.parameters.valueparameter import ValueParameter
from odxtools.physicaltype import PhysicalType
from odxtools.request import Request
from odxtools.response import Response, ResponseType
from odxtools.standardlengthtype import StandardLengthType
from odxtools.structure import Structure
from odxtools.text import Text

from helper import (
    derived_id,
    find_dop,
    functional_class_ref,
    matching_request_parameter_subfunction,
    ref,
    sid_parameter_pr,
    sid_parameter_rq,
    subfunction_rq,
)

# (code, short name, text)
DTCS = [
    (
        0xC10100,
        "HumidityTemperatureSensorNoResponse",
        "HTS221 humidity/temperature sensor no response",
    ),
    (0xC10200, "PressureSensorNoResponse", "LPS22HB pressure sensor no response"),
    (0xC10300, "InertialSensorNoResponse", "LSM6DSL inertial sensor no response"),
    (0xC10400, "MagnetometerNoResponse", "LIS2MDL magnetometer no response"),
    (0xC10500, "PresenceDetectionNotAvailable", "Occupancy data not available"),
]

DTC_STATUS_BITS = [
    "testFailed",
    "testFailedThisOperationCycle",
    "pendingDTC",
    "confirmedDTC",
    "testNotCompletedSinceLastClear",
    "testFailedSinceLastClear",
    "testNotCompletedThisOperationCycle",
    "warningIndicatorRequested",
]


def dtc_status_parameters(base: DiagLayerRaw, byte_position: int) -> list[ValueParameter]:
    true_false = ref(find_dop(base, "TrueFalseDop"))
    return [
        ValueParameter(
            short_name=name,
            semantic="DATA",
            byte_position=byte_position,
            bit_position=bit,
            dop_ref=true_false,
        )
        for bit, name in enumerate(DTC_STATUS_BITS)
    ]


def add_dtc_read_services(base: DiagLayerRaw, dlr: DiagLayerRaw):
    """19 02 <mask> -> 59 02 <availability mask> [<dtc:3> <status:1>]..."""
    ddds = dlr.diag_data_dictionary_spec

    dtc_dop = DtcDop(
        odx_id=derived_id(dlr, "DOP.RecordDataType"),
        short_name="RecordDataType",
        compu_method=CompuMethod(
            category=CompuCategory.IDENTICAL,
            physical_type=DataType.A_UINT32,
            internal_type=DataType.A_UINT32,
        ),
        physical_type=PhysicalType(base_data_type=DataType.A_UINT32),
        diag_coded_type=StandardLengthType(base_data_type=DataType.A_UINT32, bit_length=24),
        dtcs_raw=[
            DiagnosticTroubleCode(
                odx_id=derived_id(dlr, f"DTC.{short_name}"),
                short_name=short_name,
                trouble_code=code,
                display_trouble_code=f"{code:06X}",
                text=Text(text=text),
            )
            for code, short_name, text in DTCS
        ],
    )
    ddds.dtc_dops.append(dtc_dop)

    dtc_record = Structure(
        odx_id=derived_id(dlr, "STRUCT.DTCRecord"),
        short_name="DTCRecord",
        parameters=NamedItemList(
            [
                ValueParameter(
                    short_name="DTCRecord",
                    semantic="DATA",
                    byte_position=0,
                    dop_ref=ref(dtc_dop),
                ),
                *dtc_status_parameters(base, 3),
            ]
        ),
    )
    ddds.structures.append(dtc_record)

    dtc_records = EndOfPduField(
        odx_id=derived_id(dlr, "EndOfPdu.DTCRecords"),
        short_name="DTCRecords",
        structure_ref=ref(dtc_record),
    )
    ddds.end_of_pdu_fields.append(dtc_records)

    name = "FaultMem_ReportDTCByStatusMask"
    request = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{name}"),
        short_name=f"RQ_{name}",
        parameters=NamedItemList(
            [
                sid_parameter_rq(0x19),
                subfunction_rq(0x02, "SubFunction"),
                *dtc_status_parameters(base, 2),
            ]
        ),
    )
    dlr.requests.append(request)

    # Byte 2 of the response is the DTCStatusAvailabilityMask (always FF);
    # it is decoded with the same status bit layout as the reference does.
    response = Response(
        response_type=ResponseType.POSITIVE,
        odx_id=derived_id(dlr, f"PR.PR_{name}"),
        short_name=f"PR_{name}",
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x19 + 0x40),
                matching_request_parameter_subfunction("SubFunction"),
                *dtc_status_parameters(base, 2),
                ValueParameter(
                    short_name="DTCAndStatusRecord",
                    semantic="DATA",
                    byte_position=3,
                    dop_ref=ref(dtc_records),
                ),
            ]
        ),
    )
    dlr.positive_responses.append(response)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{name}"),
            short_name=name,
            long_name="Report DTC By Status Mask",
            functional_class_refs=[functional_class_ref(base, "FaultMem")],
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
        )
    )


def add_dtc_clear_services(base: DiagLayerRaw, dlr: DiagLayerRaw):
    """14 <groupOfDTC:3> -> 54. The ECU clears everything for any group;
    FF FF FF is the canonical "all groups" value (the CDA sends it itself
    when deleting all faults). Modelled like the reference with the DTC DOP,
    which cannot carry a physical default value."""
    dtc_dop = dlr.diag_data_dictionary_spec.dtc_dops["RecordDataType"]
    name = "FaultMem_ClearDTCs"

    request = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{name}"),
        short_name=f"RQ_{name}",
        parameters=NamedItemList(
            [
                sid_parameter_rq(0x14),
                ValueParameter(
                    short_name="Dtc",
                    semantic="DATA",
                    byte_position=1,
                    dop_ref=ref(dtc_dop),
                ),
            ]
        ),
    )
    dlr.requests.append(request)

    response = Response(
        response_type=ResponseType.POSITIVE,
        odx_id=derived_id(dlr, f"PR.PR_{name}"),
        short_name=f"PR_{name}",
        parameters=NamedItemList([sid_parameter_pr(0x14 + 0x40)]),
    )
    dlr.positive_responses.append(response)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{name}"),
            short_name=name,
            long_name="Clear DTCs",
            functional_class_refs=[functional_class_ref(base, "FaultMem")],
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
        )
    )
