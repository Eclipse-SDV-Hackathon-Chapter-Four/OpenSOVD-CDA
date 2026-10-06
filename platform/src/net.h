/*
 * SPDX-License-Identifier: MIT
 */

#ifndef AZ3166_NET_H
#define AZ3166_NET_H

#include "tx_api.h"

/* Wi-Fi join, DHCP and DoIP sockets. Blocks until an address is assigned.
 * Returns NX_SUCCESS (0) or a NetX error. */
UINT net_init(const char* ssid, const char* password);

#endif
