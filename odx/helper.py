# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
#
# Small builders shared by all generator modules. Adapted from the Eclipse
# OpenSOVD Classic Diagnostic Adapter test container
# (testcontainer/odx/helper.py, Apache-2.0).

from odxtools.compumethods.compucategory import CompuCategory
from odxtools.compumethods.compuconst import CompuConst
from odxtools.compumethods.compuinternaltophys import CompuInternalToPhys
from odxtools.compumethods.compuscale import CompuScale
from odxtools.compumethods.identicalcompumethod import IdenticalCompuMethod
from odxtools.compumethods.limit import Limit
from odxtools.compumethods.texttablecompumethod import TexttableCompuMethod
from odxtools.dataobjectproperty import DataObjectProperty
from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.element import IdentifiableElement
from odxtools.functionalclass import FunctionalClass
from odxtools.nameditemlist import NamedItemList
from odxtools.odxlink import OdxLinkId, OdxLinkRef
from odxtools.odxtypes import DataType
from odxtools.parameters.codedconstparameter import CodedConstParameter
from odxtools.parameters.matchingrequestparameter import MatchingRequestParameter
from odxtools.physicaltype import PhysicalType
from odxtools.preconditionstateref import PreConditionStateRef
from odxtools.radix import Radix
from odxtools.standardlengthtype import StandardLengthType
from odxtools.state import State
from odxtools.statetransition import StateTransition
from odxtools.statetransitionref import StateTransitionRef
from odxtools.unit import Unit

# --------------------------------------------------------------------------
# ODX links
# --------------------------------------------------------------------------


def derived_id(parent: OdxLinkId | IdentifiableElement, name: str) -> OdxLinkId:
    if isinstance(parent, IdentifiableElement):
        parent = parent.odx_id
    return OdxLinkId(local_id=f"{parent.local_id}.{name}", doc_fragments=parent.doc_fragments)


def ref(element: OdxLinkId | IdentifiableElement) -> OdxLinkRef:
    if isinstance(element, IdentifiableElement):
        element = element.odx_id
    return OdxLinkRef.from_id(element)


def state_transition_ref(stt: StateTransition) -> StateTransitionRef:
    return StateTransitionRef(ref_id=stt.odx_id.local_id, ref_docs=stt.odx_id.doc_fragments)


def pre_condition_state_ref(state: State) -> PreConditionStateRef:
    return PreConditionStateRef(ref_id=state.odx_id.local_id, ref_docs=state.odx_id.doc_fragments)


# --------------------------------------------------------------------------
# Lookups (always against the base variant, which owns all shared objects)
# --------------------------------------------------------------------------


def find_dop(dlr: DiagLayerRaw, short_name: str) -> DataObjectProperty:
    for dop in dlr.diag_data_dictionary_spec.data_object_props:
        if dop.short_name == short_name:
            return dop
    raise ValueError(f"DOP {short_name} not found in {dlr.short_name}")


def find_unit(dlr: DiagLayerRaw, short_name: str) -> Unit:
    unit_spec = dlr.diag_data_dictionary_spec.unit_spec
    if unit_spec is not None:
        for unit in unit_spec.units:
            if unit.short_name == short_name:
                return unit
    raise ValueError(f"Unit {short_name} not found in {dlr.short_name}")


def find_functional_class(dlr: DiagLayerRaw, short_name: str) -> FunctionalClass:
    for fc in dlr.functional_classes:
        if fc.short_name == short_name:
            return fc
    raise ValueError(f"Functional class {short_name} not found in {dlr.short_name}")


def functional_class_ref(dlr: DiagLayerRaw, short_name: str) -> OdxLinkRef:
    return ref(find_functional_class(dlr, short_name))


def find_state(dlr: DiagLayerRaw, chart: str, state: str) -> State:
    return dlr.state_charts[chart].states[state]


def find_state_transition(dlr: DiagLayerRaw, chart: str, name: str) -> StateTransition:
    for stt in dlr.state_charts[chart].state_transitions:
        if stt.short_name == name:
            return stt
    raise ValueError(f"State transition {chart}.{name} not found")


# --------------------------------------------------------------------------
# Parameters
# --------------------------------------------------------------------------


def coded_const_int_parameter(
    short_name: str,
    semantic: str,
    byte_position: int,
    coded_value_raw: str,
    bit_length: int = 8,
    bit_position: int | None = None,
) -> CodedConstParameter:
    return CodedConstParameter(
        short_name=short_name,
        semantic=semantic,
        byte_position=byte_position,
        bit_position=bit_position,
        coded_value_raw=coded_value_raw,
        diag_coded_type=StandardLengthType(base_data_type=DataType.A_UINT32, bit_length=bit_length),
    )


def sid_parameter_rq(sid: int) -> CodedConstParameter:
    return coded_const_int_parameter("SID_RQ", "SERVICE-ID", 0, str(sid))


def sid_parameter_pr(sid: int) -> CodedConstParameter:
    return coded_const_int_parameter("SID_PR", "SERVICE-ID", 0, str(sid))


def did_parameter_rq(did: int) -> CodedConstParameter:
    return coded_const_int_parameter("DID_RQ", "DID", 1, str(did), bit_length=16)


def subfunction_rq(
    subfunction: int,
    short_name: str = "SUBFUNCTION",
    semantic: str = "SUBFUNCTION",
    byte_position: int = 1,
) -> CodedConstParameter:
    return coded_const_int_parameter(short_name, semantic, byte_position, str(subfunction))


def matching_request_parameter(
    short_name: str,
    semantic: str,
    byte_length: int,
    byte_position: int = 1,
    request_byte_position: int = 1,
) -> MatchingRequestParameter:
    return MatchingRequestParameter(
        short_name=short_name,
        semantic=semantic,
        byte_position=byte_position,
        request_byte_position=request_byte_position,
        byte_length=byte_length,
    )


def matching_request_parameter_subfunction(short_name: str) -> MatchingRequestParameter:
    return matching_request_parameter(short_name, "SUBFUNCTION", 1)


def matching_request_parameter_did(short_name: str = "DID_PR") -> MatchingRequestParameter:
    return matching_request_parameter(short_name, "DID", 2)


# --------------------------------------------------------------------------
# Data object properties
# --------------------------------------------------------------------------


def identical_dop(
    dlr: DiagLayerRaw,
    short_name: str,
    base_data_type: DataType,
    bit_length: int,
    physical_type: DataType | None = None,
    unit: Unit | None = None,
    display_radix: Radix | None = None,
) -> DataObjectProperty:
    """Fixed-length DOP whose physical value equals the coded value."""
    phys = physical_type or base_data_type
    return DataObjectProperty(
        odx_id=derived_id(dlr, f"DOP.{short_name}"),
        short_name=short_name,
        compu_method=IdenticalCompuMethod(
            category=CompuCategory.IDENTICAL,
            physical_type=phys,
            internal_type=phys,
        ),
        diag_coded_type=StandardLengthType(base_data_type=base_data_type, bit_length=bit_length),
        physical_type=PhysicalType(base_data_type=phys, display_radix=display_radix),
        unit_ref=ref(unit) if unit is not None else None,
    )


def compuscales_int_to_str_map(values: list[tuple[int, str]]) -> list[CompuScale]:
    return [
        CompuScale(
            lower_limit=Limit(value_raw=str(raw), value_type=DataType.A_UINT32),
            upper_limit=Limit(value_raw=str(raw), value_type=DataType.A_UINT32),
            compu_const=CompuConst(vt=text, data_type=DataType.A_UNICODE2STRING),
            domain_type=DataType.A_UINT32,
            range_type=DataType.A_UNICODE2STRING,
        )
        for raw, text in values
    ]


def texttable_int_str_dop(
    dlr: DiagLayerRaw,
    short_name: str,
    text_table: list[tuple[int, str]],
    bit_length: int = 8,
) -> DataObjectProperty:
    return DataObjectProperty(
        odx_id=derived_id(dlr, f"DOP.{short_name}"),
        short_name=short_name,
        compu_method=TexttableCompuMethod(
            category=CompuCategory.TEXTTABLE,
            compu_internal_to_phys=CompuInternalToPhys(
                compu_scales=compuscales_int_to_str_map(text_table),
            ),
            physical_type=DataType.A_UNICODE2STRING,
            internal_type=DataType.A_UINT32,
        ),
        diag_coded_type=StandardLengthType(base_data_type=DataType.A_UINT32, bit_length=bit_length),
        physical_type=PhysicalType(base_data_type=DataType.A_UNICODE2STRING),
    )


def named(items: list) -> NamedItemList:
    return NamedItemList(items)
