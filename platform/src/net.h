/*
 * SPDX-License-Identifier: MIT
 */

#ifndef FLXC1000_NET_H
#define FLXC1000_NET_H

#include "tx_api.h"

/* Wi-Fi join, DHCP and DoIP sockets. Blocks until an address is assigned.
 * Returns NX_SUCCESS (0) or a NetX error. */
UINT net_init(const char* ssid, const char* password);

#endif
