# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# RoutineControl (0x31) services. Modelled like the Eclipse OpenSOVD Classic
# Diagnostic Adapter test container (testcontainer/odx/routines.py,
# Apache-2.0). docs/diagnostics.md "Routines (App)".

from odxtools.compumethods.compumethod import CompuCategory
from odxtools.compumethods.identicalcompumethod import IdenticalCompuMethod
from odxtools.dataobjectproperty import DataObjectProperty
from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.diagservice import DiagService
from odxtools.encoding import Encoding
from odxtools.minmaxlengthtype import MinMaxLengthType
from odxtools.nameditemlist import NamedItemList
from odxtools.parameters.parameter import Parameter
from odxtools.parameters.valueparameter import ValueParameter
from odxtools.physicaltype import PhysicalType
from odxtools.request import Request
from odxtools.response import Response, ResponseType
from odxtools.termination import Termination
from odxtools.odxtypes import DataType

from helper import (
    coded_const_int_parameter,
    derived_id,
    functional_class_ref,
    matching_request_parameter,
    matching_request_parameter_subfunction,
    pre_condition_state_ref,
    ref,
    sid_parameter_pr,
    sid_parameter_rq,
    subfunction_rq,
    find_dop,
    texttable_int_str_dop,
)

ROUTINE_TYPE_TO_SUBFUNCTION = {"Start": 0x01, "Stop": 0x02, "RequestResults": 0x03}

# Longest SpeakText text (crates/az3166-ecu/src/board.rs MAX_SPEECH_TEXT)
MAX_SPEECH_TEXT = 200


def add_routine(
    base: DiagLayerRaw,
    dlr: DiagLayerRaw,
    name: str,
    routine_id: int,
    routine_type: str,
    response_params: list[Parameter] | None = None,
    description: str | None = None,
    request_params: list[Parameter] | None = None,
    sessions: list[str] | None = None,
):
    """31 <type> <rid:2> [request_params] -> 71 <type> <rid:2> [response_params]"""
    subfunction = ROUTINE_TYPE_TO_SUBFUNCTION[routine_type]
    service_name = f"{name}_{routine_type}"

    request = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{service_name}"),
        short_name=f"RQ_{service_name}",
        parameters=NamedItemList(
            [
                sid_parameter_rq(0x31),
                subfunction_rq(subfunction, "RoutineControlType"),
                coded_const_int_parameter("RoutineId", "DATA", 2, str(routine_id), 16),
                *(request_params or []),
            ]
        ),
    )
    dlr.requests.append(request)

    response = Response(
        response_type=ResponseType.POSITIVE,
        odx_id=derived_id(dlr, f"PR.PR_{service_name}"),
        short_name=f"PR_{service_name}",
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x31 + 0x40),
                matching_request_parameter_subfunction("RoutineControlType"),
                matching_request_parameter("RoutineId", "DATA", 2, 2, 2),
                *(response_params or []),
            ]
        ),
    )
    dlr.positive_responses.append(response)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{service_name}"),
            short_name=service_name,
            long_name=description,
            functional_class_refs=[functional_class_ref(base, "Routines")],
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
            pre_condition_state_refs=[
                pre_condition_state_ref(base.state_charts["Session"].states[s])
                for s in (sessions or [])
            ],
        )
    )


def add_routine_control_services(base: DiagLayerRaw, dlr: DiagLayerRaw):
    status_dop = texttable_int_str_dop(
        dlr,
        "RoutineStatus",
        [(0x00, "Idle"), (0x01, "Running"), (0x02, "Completed"), (0x03, "Aborted")],
    )
    dlr.diag_data_dictionary_spec.data_object_props.append(status_dop)

    def status_param():
        return [
            ValueParameter(
                short_name="RoutineStatus",
                semantic="DATA",
                byte_position=4,
                dop_ref=ref(status_dop),
            )
        ]

    # 31 01 10 01 -> 71 01 10 01 01 (LED cascade, ~1.5 s)
    # 31 02 10 01 -> 71 02 10 01 03
    # 31 03 10 01 -> 71 03 10 01 <status>
    for routine_type, description in [
        ("Start", "Self Test (LED cascade)"),
        ("Stop", "Self Test Stop"),
        ("RequestResults", "Self Test Request Results"),
    ]:
        add_routine(
            base,
            dlr,
            name="SelfTest",
            routine_id=0x1001,
            routine_type=routine_type,
            response_params=status_param(),
            description=description,
            sessions=["Extended"],
        )

    # 31 01 10 02 -> 71 01 10 02 01 (speaks the ambient temperature on the
    # headphone jack: "The temperature is 23.5 degrees Celsius.")
    # 31 02 10 02 -> 71 02 10 02 03
    # 31 03 10 02 -> 71 03 10 02 <status>
    for routine_type, description in [
        ("Start", "Announce Temperature"),
        ("Stop", "Announce Temperature Stop"),
        ("RequestResults", "Announce Temperature Request Results"),
    ]:
        add_routine(
            base,
            dlr,
            name="AnnounceTemperature",
            routine_id=0x1002,
            routine_type=routine_type,
            response_params=status_param(),
            description=description,
            sessions=["Default", "Extended"],
        )

    # 31 01 10 05 <text> -> 71 01 10 05 01 (speaks 1..200 bytes of printable
    # ASCII on the headphone jack; replaces a running AnnounceTemperature)
    # 31 02 10 05 -> 71 02 10 05 03
    # 31 03 10 05 -> 71 03 10 05 <status>
    text_dop = DataObjectProperty(
        odx_id=derived_id(dlr, "DOP.SpeechText"),
        short_name="SpeechText",
        compu_method=IdenticalCompuMethod(
            category=CompuCategory.IDENTICAL,
            physical_type=DataType.A_UNICODE2STRING,
            internal_type=DataType.A_UNICODE2STRING,
        ),
        diag_coded_type=MinMaxLengthType(
            base_data_type=DataType.A_ASCIISTRING,
            base_type_encoding=Encoding.ISO_8859_1,
            min_length=1,
            max_length=MAX_SPEECH_TEXT,
            termination=Termination.END_OF_PDU,
        ),
        physical_type=PhysicalType(base_data_type=DataType.A_UNICODE2STRING),
    )
    dlr.diag_data_dictionary_spec.data_object_props.append(text_dop)
    for routine_type, description in [
        ("Start", "Speak Text"),
        ("Stop", "Speak Text Stop"),
        ("RequestResults", "Speak Text Request Results"),
    ]:
        add_routine(
            base,
            dlr,
            name="SpeakText",
            routine_id=0x1005,
            routine_type=routine_type,
            request_params=(
                [
                    ValueParameter(
                        short_name="Text",
                        semantic="DATA",
                        byte_position=4,
                        dop_ref=ref(text_dop),
                    )
                ]
                if routine_type == "Start"
                else None
            ),
            response_params=status_param(),
            description=description,
            sessions=["Default", "Extended"],
        )

    # Volume one step (10 %) up / down. Start only, so the CDA runs them
    # synchronously and returns the new volume.
    # 31 01 10 03 -> 71 01 10 03 <volume %>
    # 31 01 10 04 -> 71 01 10 04 <volume %>
    volume_dop = find_dop(dlr, "Percent_UInt8")
    for name, rid, description in [
        ("VolumeUp", 0x1003, "Volume Up (+10 %)"),
        ("VolumeDown", 0x1004, "Volume Down (-10 %, 0 = mute)"),
    ]:
        add_routine(
            base,
            dlr,
            name=name,
            routine_id=rid,
            routine_type="Start",
            response_params=[
                ValueParameter(
                    short_name="Volume",
                    semantic="DATA",
                    byte_position=4,
                    dop_ref=ref(volume_dop),
                )
            ],
            description=description,
            sessions=["Default", "Extended"],
        )
