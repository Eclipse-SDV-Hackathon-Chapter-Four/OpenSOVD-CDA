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
# Administrative data, company data, audiences and functional classes.
# Structure follows the Eclipse OpenSOVD Classic Diagnostic Adapter test
# container (testcontainer/odx/metadata.py, Apache-2.0).

import datetime
import os

from odxtools.additionalaudience import AdditionalAudience
from odxtools.admindata import AdminData
from odxtools.companydata import CompanyData
from odxtools.description import Description
from odxtools.diaglayercontainer import DiagLayerContainer
from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.docrevision import DocRevision
from odxtools.functionalclass import FunctionalClass
from odxtools.nameditemlist import NamedItemList
from odxtools.odxlink import OdxLinkId

from helper import derived_id

REVISION_LABEL = "00.01.00"

# Functional classes. The names match the reference database because the CDA
# looks some of them up by name (e.g. "FaultMem" for 0x14 / 0x19).
FUNCTIONAL_CLASSES = [
    "Session",
    "EcuReset",
    "Ident",
    "CurrentData",
    "StoredData",
    "flash_download_upload",
    "SecurityAccess",
    "FaultMem",
    "Routines",
    "TesterPresent",
]


def _build_timestamp() -> str:
    # Honour SOURCE_DATE_EPOCH so that rebuilds can be byte-reproducible.
    epoch = os.environ.get("SOURCE_DATE_EPOCH")
    if epoch is not None:
        return datetime.datetime.fromtimestamp(int(epoch), tz=datetime.UTC).isoformat()
    return datetime.datetime.now(tz=datetime.UTC).isoformat()


def add_admin_data(dlc: DiagLayerContainer):
    dlc.admin_data = AdminData(
        doc_revisions=[DocRevision(revision_label=REVISION_LABEL, date=_build_timestamp())]
    )


def add_company_datas(dlc: DiagLayerContainer):
    dlc.company_datas = NamedItemList(
        [
            CompanyData(
                odx_id=OdxLinkId("CD.AZ3166", doc_fragments=dlc.odx_id.doc_fragments),
                short_name="AZ3166",
                long_name="AZ3166 ECU simulator for the MXCHIP AZ3166 IoT DevKit",
                description=Description.from_string(
                    "UDS-over-DoIP ECU simulator (STM32F412, ThreadX, DoIP over Wi-Fi)"
                ),
            )
        ]
    )


def add_additional_audiences(dlc: DiagLayerContainer):
    names = ["Anyone", "After_Sales", "Manufacturing", "Development", "Supplier"]
    dlc.additional_audiences = NamedItemList(
        [AdditionalAudience(odx_id=derived_id(dlc, f"AA.{n}"), short_name=n) for n in names]
    )


def add_functional_classes(dlr: DiagLayerRaw):
    dlr.functional_classes = NamedItemList(
        [
            FunctionalClass(odx_id=derived_id(dlr, f"FNC.{name}"), short_name=name)
            for name in FUNCTIONAL_CLASSES
        ]
    )
