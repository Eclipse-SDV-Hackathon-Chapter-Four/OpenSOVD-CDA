/*
 * SPDX-License-Identifier: MIT
 *
 * Wi-Fi (WICED, BCM43362) + NetX Duo IPv4/DHCP + DoIP socket shims.
 * Network bring-up derived from wwd_networking.c of the Azure RTOS
 * getting-started guides, Copyright (c) Microsoft Corporation.
 */

#include "net.h"

#include <stdio.h>
#include <string.h>

#include "nx_api.h"
#include "nxd_dhcp_client.h"
#include "wiced_sdk.h"

#include "board.h"
#include "platform.h"

#define DOIP_PORT 13400

#define NETX_IP_STACK_SIZE   2048
#define NETX_TX_PACKET_COUNT 8
#define NETX_RX_PACKET_COUNT 8
#define NETX_PACKET_SIZE     (WICED_LINK_MTU)
#define NETX_TX_POOL_SIZE    ((NETX_PACKET_SIZE + sizeof(NX_PACKET)) * NETX_TX_PACKET_COUNT)
#define NETX_RX_POOL_SIZE    ((NETX_PACKET_SIZE + sizeof(NX_PACKET)) * NETX_RX_PACKET_COUNT)
#define NETX_ARP_CACHE_SIZE  512

#define TCP_WINDOW_SIZE  4096
#define TCP_SEND_CHUNK   1024
#define TCP_LISTEN_QUEUE 2
#define UDP_QUEUE_MAX    4

#define DHCP_WAIT_TICKS (30 * TX_TIMER_TICKS_PER_SECOND)
#define WIFI_COUNTRY    WICED_COUNTRY_WORLD_WIDE_XX

static UCHAR netx_ip_stack[NETX_IP_STACK_SIZE];
static UCHAR netx_tx_pool_area[NETX_TX_POOL_SIZE];
static UCHAR netx_rx_pool_area[NETX_RX_POOL_SIZE];
static ULONG netx_arp_cache_area[NETX_ARP_CACHE_SIZE / sizeof(ULONG)];

static NX_IP nx_ip;
static NX_PACKET_POOL nx_pool[2]; /* 0 = TX, 1 = RX: layout expected by WICED */
static NX_DHCP nx_dhcp_client;

static uint8_t mac_address[6];
static volatile ULONG ip_address;

static NX_UDP_SOCKET udp_socket;

typedef struct
{
    NX_TCP_SOCKET socket;
    NX_PACKET* pending; /* partially consumed received packet */
    ULONG pending_offset;
} tcp_slot_t;

static tcp_slot_t tcp_slots[PLAT_TCP_SLOTS];

/* A NetX listen entry holds one socket at a time. The slot holding this
 * mutex owns the listener: it (re)listens with its socket, waits for a
 * tester, and hands the listener over once the connection is accepted
 * (NetX frees the listen entry when the SYN arrives). */
static TX_MUTEX listen_mutex;
static int listening;

static ULONG ms_to_ticks(uint32_t ms)
{
    return (ms * TX_TIMER_TICKS_PER_SECOND + 999u) / 1000u;
}

static UINT wifi_init(void)
{
    wiced_mac_t mac;

    if (wwd_buffer_init(nx_pool) != WWD_SUCCESS)
    {
        printf("ERROR: wwd_buffer_init\r\n");
        return NX_NOT_SUCCESSFUL;
    }
    if (wwd_management_wifi_on(WIFI_COUNTRY) != WWD_SUCCESS)
    {
        printf("ERROR: wwd_management_wifi_on\r\n");
        return NX_NOT_SUCCESSFUL;
    }
    wwd_wifi_get_mac_address(&mac, WWD_STA_INTERFACE);
    memcpy(mac_address, mac.octet, sizeof(mac_address));
    printf("Wi-Fi MAC %02X:%02X:%02X:%02X:%02X:%02X\r\n",
        mac.octet[0], mac.octet[1], mac.octet[2], mac.octet[3], mac.octet[4], mac.octet[5]);
    return NX_SUCCESS;
}

static void wifi_join(const char* ssid, const char* password)
{
    wiced_ssid_t wiced_ssid = {0};
    wiced_security_t security = password[0] ? WICED_SECURITY_WPA2_AES_PSK : WICED_SECURITY_OPEN;

    wiced_ssid.length = (uint8_t)strnlen(ssid, sizeof(wiced_ssid.value));
    memcpy(wiced_ssid.value, ssid, wiced_ssid.length);

    for (int attempt = 1;; attempt++)
    {
        printf("Joining '%s' (attempt %d)\r\n", ssid, attempt);

        /* Hold the IP mutex while (re)joining, as the WICED driver expects. */
        tx_mutex_get(&nx_ip.nx_ip_protection, TX_WAIT_FOREVER);
        wwd_result_t result = wwd_wifi_join(
            &wiced_ssid, security, (const uint8_t*)password, (uint8_t)strlen(password), NULL, WWD_STA_INTERFACE);
        tx_mutex_put(&nx_ip.nx_ip_protection);

        if (result == WWD_SUCCESS)
        {
            return;
        }
        printf("Join failed (%d)\r\n", (int)result);
        tx_thread_sleep(5 * TX_TIMER_TICKS_PER_SECOND);
    }
}

static UINT dhcp_wait(void)
{
    ULONG actual_status;
    ULONG mask;
    char text[20];

    while (nx_ip_status_check(&nx_ip, NX_IP_ADDRESS_RESOLVED, &actual_status, DHCP_WAIT_TICKS) != NX_SUCCESS)
    {
        printf("Waiting for DHCP\r\n");
    }

    ULONG address;
    nx_ip_address_get(&nx_ip, &address, &mask);
    snprintf(text, sizeof(text), "%lu.%lu.%lu.%lu",
        address >> 24, (address >> 16) & 0xFF, (address >> 8) & 0xFF, address & 0xFF);
    printf("IP address %s\r\n", text);
    ip_address = address;
    WIFI_LED_ON();
    return NX_SUCCESS;
}

static UINT sockets_init(void)
{
    UINT status;

    if ((status = nx_udp_socket_create(
             &nx_ip, &udp_socket, "doip-udp", NX_IP_NORMAL, NX_FRAGMENT_OKAY, NX_IP_TIME_TO_LIVE, UDP_QUEUE_MAX)))
    {
        return status;
    }
    if ((status = nx_udp_socket_bind(&udp_socket, DOIP_PORT, TX_WAIT_FOREVER)))
    {
        return status;
    }

    for (uint32_t i = 0; i < PLAT_TCP_SLOTS; i++)
    {
        if ((status = nx_tcp_socket_create(&nx_ip, &tcp_slots[i].socket, "doip-tcp", NX_IP_NORMAL,
                 NX_FRAGMENT_OKAY, NX_IP_TIME_TO_LIVE, TCP_WINDOW_SIZE, NX_NULL, NX_NULL)))
        {
            return status;
        }
    }

    return tx_mutex_create(&listen_mutex, "doip-listen", TX_INHERIT);
}

UINT net_init(const char* ssid, const char* password)
{
    UINT status;

    if (ssid[0] == '\0')
    {
        printf("ERROR: no Wi-Fi SSID configured (see README)\r\n");
        return NX_NOT_SUCCESSFUL;
    }

    nx_system_initialize();

    if ((status = nx_packet_pool_create(
             &nx_pool[0], "tx-pool", NETX_PACKET_SIZE, netx_tx_pool_area, NETX_TX_POOL_SIZE)) ||
        (status = nx_packet_pool_create(
             &nx_pool[1], "rx-pool", NETX_PACKET_SIZE, netx_rx_pool_area, NETX_RX_POOL_SIZE)))
    {
        printf("ERROR: nx_packet_pool_create (0x%02x)\r\n", status);
        return status;
    }

    if ((status = wifi_init()))
    {
        return status;
    }

    if ((status = nx_ip_create(&nx_ip, "ip", IP_ADDRESS(0, 0, 0, 0), IP_ADDRESS(255, 255, 255, 0), &nx_pool[0],
             wiced_sta_netx_duo_driver_entry, netx_ip_stack, NETX_IP_STACK_SIZE, 1)) ||
        (status = nx_arp_enable(&nx_ip, netx_arp_cache_area, sizeof(netx_arp_cache_area))) ||
        (status = nx_tcp_enable(&nx_ip)) || (status = nx_udp_enable(&nx_ip)) || (status = nx_icmp_enable(&nx_ip)))
    {
        printf("ERROR: NetX IP setup (0x%02x)\r\n", status);
        return status;
    }

    wifi_join(ssid, password);

    if ((status = nx_dhcp_create(&nx_dhcp_client, &nx_ip, "az3166")) ||
        (status = nx_dhcp_start(&nx_dhcp_client)))
    {
        printf("ERROR: DHCP (0x%02x)\r\n", status);
        return status;
    }
    dhcp_wait();

    if ((status = sockets_init()))
    {
        printf("ERROR: DoIP sockets (0x%02x)\r\n", status);
        return status;
    }
    printf("DoIP listening on UDP/TCP %d\r\n", DOIP_PORT);
    return NX_SUCCESS;
}

/* ---- platform API -------------------------------------------------------- */

void plat_net_mac(uint8_t out[6])
{
    memcpy(out, mac_address, sizeof(mac_address));
}

void plat_net_ip(uint8_t out[4])
{
    ULONG address = ip_address;
    out[0] = (uint8_t)(address >> 24);
    out[1] = (uint8_t)(address >> 16);
    out[2] = (uint8_t)(address >> 8);
    out[3] = (uint8_t)address;
}

int32_t plat_udp_recv(uint8_t* buf, uint32_t cap, uint32_t* ip, uint16_t* port, uint32_t timeout_ms)
{
    NX_PACKET* packet;
    UINT source_port;
    ULONG copied = 0;

    UINT status = nx_udp_socket_receive(&udp_socket, &packet, ms_to_ticks(timeout_ms));
    if (status == NX_NO_PACKET)
    {
        return PLAT_TIMEOUT;
    }
    if (status != NX_SUCCESS)
    {
        return PLAT_ERROR;
    }
    nx_udp_source_extract(packet, (ULONG*)ip, &source_port);
    *port = (uint16_t)source_port;
    nx_packet_data_extract_offset(packet, 0, buf, cap, &copied);
    nx_packet_release(packet);
    return (int32_t)copied;
}

static NX_PACKET* packet_from(const uint8_t* buf, uint32_t len, ULONG packet_type)
{
    NX_PACKET* packet;
    if (nx_packet_allocate(&nx_pool[0], &packet, packet_type, NX_WAIT_FOREVER) != NX_SUCCESS)
    {
        return NULL;
    }
    if (nx_packet_data_append(packet, (VOID*)buf, len, &nx_pool[0], NX_WAIT_FOREVER) != NX_SUCCESS)
    {
        nx_packet_release(packet);
        return NULL;
    }
    return packet;
}

int32_t plat_udp_send(uint32_t ip, uint16_t port, const uint8_t* buf, uint32_t len)
{
    NX_PACKET* packet = packet_from(buf, len, NX_UDP_PACKET);
    if (packet == NULL)
    {
        return PLAT_ERROR;
    }
    if (nx_udp_socket_send(&udp_socket, packet, ip, port) != NX_SUCCESS)
    {
        nx_packet_release(packet);
        return PLAT_ERROR;
    }
    return 0;
}

int32_t plat_tcp_accept(uint32_t slot)
{
    if (slot >= PLAT_TCP_SLOTS)
    {
        return PLAT_ERROR;
    }
    NX_TCP_SOCKET* socket = &tcp_slots[slot].socket;

    tx_mutex_get(&listen_mutex, TX_WAIT_FOREVER);
    UINT status;
    if (!listening)
    {
        status = nx_tcp_server_socket_listen(&nx_ip, DOIP_PORT, socket, TCP_LISTEN_QUEUE, NX_NULL);
        listening = status == NX_SUCCESS;
    }
    else
    {
        status = nx_tcp_server_socket_relisten(&nx_ip, DOIP_PORT, socket);
        if (status == NX_CONNECTION_PENDING)
        {
            status = NX_SUCCESS;
        }
    }
    if (status == NX_SUCCESS)
    {
        status = nx_tcp_server_socket_accept(socket, NX_WAIT_FOREVER);
        if (status != NX_SUCCESS)
        {
            nx_tcp_server_socket_unaccept(socket);
        }
    }
    tx_mutex_put(&listen_mutex);

    if (status != NX_SUCCESS)
    {
        printf("DoIP accept failed on slot %lu (0x%02x)\r\n", (unsigned long)slot, status);
        return PLAT_ERROR;
    }
    return 0;
}

int32_t plat_tcp_recv(uint32_t slot, uint8_t* buf, uint32_t cap, uint32_t timeout_ms)
{
    tcp_slot_t* s = &tcp_slots[slot];

    if (s->pending == NULL)
    {
        UINT status = nx_tcp_socket_receive(&s->socket, &s->pending, ms_to_ticks(timeout_ms));
        if (status == NX_NO_PACKET)
        {
            return PLAT_TIMEOUT;
        }
        if (status != NX_SUCCESS)
        {
            s->pending = NULL;
            return PLAT_CLOSED;
        }
        s->pending_offset = 0;
    }

    ULONG copied = 0;
    nx_packet_data_extract_offset(s->pending, s->pending_offset, buf, cap, &copied);
    s->pending_offset += copied;
    if (s->pending_offset >= s->pending->nx_packet_length)
    {
        nx_packet_release(s->pending);
        s->pending = NULL;
    }
    return (int32_t)copied;
}

int32_t plat_tcp_send(uint32_t slot, const uint8_t* buf, uint32_t len)
{
    tcp_slot_t* s = &tcp_slots[slot];

    while (len > 0)
    {
        uint32_t chunk    = len > TCP_SEND_CHUNK ? TCP_SEND_CHUNK : len;
        NX_PACKET* packet = packet_from(buf, chunk, NX_TCP_PACKET);
        if (packet == NULL)
        {
            return PLAT_ERROR;
        }
        if (nx_tcp_socket_send(&s->socket, packet, 5 * TX_TIMER_TICKS_PER_SECOND) != NX_SUCCESS)
        {
            nx_packet_release(packet);
            return PLAT_ERROR;
        }
        buf += chunk;
        len -= chunk;
    }
    return 0;
}

void plat_tcp_close(uint32_t slot)
{
    tcp_slot_t* s = &tcp_slots[slot];

    if (s->pending != NULL)
    {
        nx_packet_release(s->pending);
        s->pending = NULL;
    }
    nx_tcp_socket_disconnect(&s->socket, TX_TIMER_TICKS_PER_SECOND);
    nx_tcp_server_socket_unaccept(&s->socket);
}
