# SPDX-License-Identifier: Apache-2.0
# This file is 100% AI-generated (Claude Code, Claude Opus 5.5).
"""Minimal SOVD client for the AZ3166 behind the OpenSOVD CDA (stdlib only).

Used by the az3166-presence-* scripts, through the CDA's SOVD data and
operations endpoints (names from the AZ3166 MDD). Scaled values (0.1 degC)
need a CDA that decodes LINEAR values into the DOP's physical type
(A_FLOAT64); older CDAs truncate them to whole degrees.

Token: by default from the CDA's own /authorize (client "test"). With
AZ3166_TOKEN_CMD set, that shell command prints a bearer token (last output
line) instead, e.g. one of the client a diagnostic agent uses, so these
scripts own CDA locks together with that agent. The token is fetched once
and again only shortly before it expires or when the CDA rejects it.

Lock: by default these scripts lock the ECU themselves (short, renewed).
With AZ3166_LOCK_CMD set, that shell command takes / renews the ECU lock
instead (e.g. the agent's own lock command), so all locks come from one
place.
"""

import base64
import json
import os
import subprocess
import time
import urllib.error
import urllib.request

DEFAULT_CDA = "http://localhost:20002"
DEFAULT_ECU = "az3166"

# Data and operations (docs/diagnostics.md; DIDs F240-F244, routine 1006)
DATA_PRESENCE = "PresenceState"
DATA_ALARM = "TemperatureAlarm"
DATA_RISE = "AlarmRiseThreshold"
DATA_WINDOW = "AlarmTimeWindow"
DATA_FALL = "AlarmFallThreshold"
DATA_HOT_LIMIT = "AlarmHotLimit"
DATA_HOT_HOLD = "AlarmHotHoldTime"
OPERATION_RESET = "ResetDetection"


class CdaError(Exception):
    pass


def token_expiry(token: str) -> float:
    """`exp` of a JWT (not verified), or 0 if there is none."""
    try:
        payload = token.split(".")[1]
        payload += "=" * (-len(payload) % 4)
        return float(json.loads(base64.urlsafe_b64decode(payload)).get("exp", 0))
    except (IndexError, ValueError):
        return 0.0


class Cda:
    TOKEN_MARGIN_S = 120
    # Own ECU lock: short, renewed by connect() (the log calls it every
    # lock_renew_s): a client of the same identity (e.g. one locking for
    # 600 s) can then always extend it - the CDA rejects renewals that
    # shorten the deadline.
    LOCK_EXPIRATION_S = 120

    def __init__(self, base: str = DEFAULT_CDA, ecu: str = DEFAULT_ECU):
        self.base = base.rstrip("/") + "/vehicle/v15"
        self.ecu = ecu
        self.token = None
        self.token_cmd = os.environ.get("AZ3166_TOKEN_CMD")
        self.lock_cmd = os.environ.get("AZ3166_LOCK_CMD")
        # how often a long-running client should call connect()
        self.lock_renew_s = 240 if self.lock_cmd else 60

    def _request(self, method, path, body=None, timeout=15):
        url = self.base + path
        headers = {}
        data = None
        if body is not None:
            data = json.dumps(body).encode()
            headers["Content-Type"] = "application/json"
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        req = urllib.request.Request(url, data=data, method=method, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                status, payload = resp.status, resp.read()
        except urllib.error.HTTPError as e:
            status, payload = e.code, e.read()
        except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
            raise CdaError(f"{method} {path}: {getattr(e, 'reason', e)}") from None
        if status == 401 and path != "/authorize":
            self.token = None  # fetched again on the next connect()
        if status >= 300:
            raise CdaError(f"{method} {path}: HTTP {status} {payload.decode(errors='replace')[:200]}")
        return json.loads(payload) if payload else None

    def request(self, method, path, body=None, retries=2):
        """A request; read-only ones are retried when the CDA reports an
        "Unexpected DoIP event" (acknowledgement of a concurrent request)."""
        for attempt in range(retries + 1):
            try:
                return self._request(method, path, body)
            except CdaError as e:
                if method != "GET" or attempt == retries or "Unexpected DoIP event" not in str(e):
                    raise
                time.sleep(0.2)

    def component(self, sub=""):
        return f"/components/{self.ecu}{sub}"

    def _fetch_token(self) -> str:
        if not self.token_cmd:
            return self._request(
                "POST", "/authorize", {"client_id": "test", "client_secret": "test"}
            )["access_token"]
        result = subprocess.run(self.token_cmd, shell=True, capture_output=True, text=True)
        lines = [line.strip() for line in result.stdout.splitlines() if line.strip()]
        if result.returncode != 0 or not lines:
            raise CdaError(f"token command failed ({result.returncode}): {result.stderr.strip()[-200:]}")
        return lines[-1]

    def connect(self):
        """Token (reused while valid) and ECU lock (taken or renewed). Locks
        belong to the token's client, so several clients of the same identity
        can use the ECU; a lock of theirs running longer is kept."""
        expiry = token_expiry(self.token) if self.token else 0.0
        if not self.token or (expiry and expiry - time.time() < self.TOKEN_MARGIN_S):
            self.token = None
            self.token = self._fetch_token()
        if self.lock_cmd:
            result = subprocess.run(self.lock_cmd, shell=True, capture_output=True, text=True)
            if result.returncode != 0:
                raise CdaError(f"lock command failed ({result.returncode})")
            return
        try:
            self._request("POST", self.component("/locks"), {"lock_expiration": self.LOCK_EXPIRATION_S})
        except CdaError:
            locks = self._request("GET", self.component("/locks")) or {}
            if not any(item.get("owned") for item in locks.get("items", [])):
                raise

    def session(self, name: str):
        self._request("PUT", self.component("/modes/session"), {"value": name})

    def read_data(self, name: str) -> dict:
        return self.request("GET", self.component(f"/data/{name}"))["data"]

    def write_data(self, name: str, value):
        self._request("PUT", self.component(f"/data/{name}"), {"data": {name: value}})

    def run_operation(self, name: str) -> dict:
        """Start-only routine (synchronous in the CDA): its response parameters."""
        response = self._request("POST", self.component(f"/operations/{name}/executions"), {})
        return (response or {}).get("parameters", {})


def read_alarm(cda: Cda) -> dict:
    """Presence and alarm state, temperatures in degC."""
    presence = cda.read_data(DATA_PRESENCE)[DATA_PRESENCE]
    alarm = cda.read_data(DATA_ALARM)
    return {
        "presence": presence,
        "alarm": alarm["AlarmState"],
        "temperature": float(alarm["CurrentTemperature"]),
        "baseline": float(alarm["BaselineTemperature"]),
        "rise": float(alarm["TemperatureRise"]),
    }


def read_config(cda: Cda) -> tuple[float, int, float]:
    """(rise threshold in degC, time window in s, fall threshold in degC)"""
    rise = float(cda.read_data(DATA_RISE)[DATA_RISE])
    window = int(cda.read_data(DATA_WINDOW)[DATA_WINDOW])
    fall = float(cda.read_data(DATA_FALL)[DATA_FALL])
    return rise, window, fall
