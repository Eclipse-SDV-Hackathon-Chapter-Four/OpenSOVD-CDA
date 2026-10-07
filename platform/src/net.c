/*
 * SPDX-License-Identifier: MIT
 *
 * Wi-Fi (WICED, BCM43362) + NetX Duo IPv4 with a link-local address
 * (169.254/16, derived from the MAC address, set at once: the vehicle network
 * has no DHCP server) + DoIP socket shims + the presence sensor client.
 * Network bring-up derived from wwd_networking.c of the Azure RTOS
 * getting-started guides, Copyright (c) Microsoft Corporation.
 */

#include "net.h"

#include <stdio.h>
#include <string.h>

#include "nx_api.h"
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

/* Link checks (plat_net_maintain, once a second): rejoin after this many
 * seconds without a usable link. */
#define LINK_DOWN_REJOIN_S 3

/* Presence sensor: HTTP GET of its occupancy entity (JSON "value":true/false; the PIR motion sensor). */
#ifndef PRESENCE_SENSOR_HOST
#define PRESENCE_SENSOR_HOST "169.254.200.10"
#endif
#ifndef PRESENCE_SENSOR_PATH
#define PRESENCE_SENSOR_PATH "/binary_sensor/PIR"
#endif
#define PRESENCE_PORT       80
#define PRESENCE_TIMEOUT_MS 800
#define PRESENCE_REPLY_MAX  768
#define WIFI_COUNTRY    WICED_COUNTRY_WORLD_WIDE_XX

static UCHAR netx_ip_stack[NETX_IP_STACK_SIZE];
static UCHAR netx_tx_pool_area[NETX_TX_POOL_SIZE];
static UCHAR netx_rx_pool_area[NETX_RX_POOL_SIZE];
static ULONG netx_arp_cache_area[NETX_ARP_CACHE_SIZE / sizeof(ULONG)];

static NX_IP nx_ip;
static NX_PACKET_POOL nx_pool[2]; /* 0 = TX, 1 = RX: layout expected by WICED */
static const char* wifi_ssid;
static const char* wifi_password;

/* In the WICED library, not declared in wiced_sdk.h. */
extern wwd_result_t wwd_wifi_disable_powersave(void);

static NX_TCP_SOCKET presence_socket;
static int presence_socket_ready;

static uint8_t mac_address[6];
static volatile ULONG ip_address;
static volatile ULONG ip_mask;

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

/* Link-local address (RFC 3927 range) derived from the MAC address: the same
 * on every start, so testers can keep it. */
static UINT address_set(void)
{
    char text[20];
    ULONG address = IP_ADDRESS(169, 254, 1u + mac_address[4] % 254u, mac_address[5]);
    ULONG mask    = IP_ADDRESS(255, 255, 0, 0);
    UINT status   = nx_ip_address_set(&nx_ip, address, mask);
    if (status != NX_SUCCESS)
    {
        printf("ERROR: address (0x%02x)\r\n", status);
        return status;
    }
    nx_arp_gratuitous_send(&nx_ip, NX_NULL);
    snprintf(text, sizeof(text), "%lu.%lu.%lu.%lu",
        address >> 24, (address >> 16) & 0xFF, (address >> 8) & 0xFF, address & 0xFF);
    printf("IP address %s (link-local)\r\n", text);
    ip_mask    = mask;
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

    wifi_ssid     = ssid;
    wifi_password = password;
    wifi_join(ssid, password);
    /* Power save makes the chip miss traffic for it (seen: associated but
     * unreachable for many minutes). */
    if (wwd_wifi_disable_powersave() != WWD_SUCCESS)
    {
        printf("Wi-Fi: could not disable power save\r\n");
    }

    if ((status = address_set()))
    {
        return status;
    }

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

void plat_net_maintain(void)
{
    static uint32_t down_s;

    if (wifi_ssid == NULL)
    {
        return;
    }
    if (wwd_wifi_is_ready_to_transceive(WWD_STA_INTERFACE) == WWD_SUCCESS)
    {
        down_s = 0;
        return;
    }
    if (++down_s < LINK_DOWN_REJOIN_S)
    {
        return;
    }
    printf("Wi-Fi: link lost, rejoining\r\n");
    WIFI_LED_OFF();
    wifi_join(wifi_ssid, wifi_password);
    wwd_wifi_disable_powersave();
    nx_arp_gratuitous_send(&nx_ip, NX_NULL);
    WIFI_LED_ON();
    printf("Wi-Fi: rejoined\r\n");
    down_s = 0;
}

uint32_t plat_net_broadcast(void)
{
    /* Subnet-directed broadcast: hosts with several interfaces route it
     * (255.255.255.255 leaves through their primary interface only). */
    return ip_address == 0 ? 0xFFFFFFFFu : (uint32_t)(ip_address | ~ip_mask);
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

/* ---- presence sensor ----------------------------------------------------- */

static int parse_ipv4(const char* text, ULONG* address)
{
    ULONG value = 0;
    for (int part = 0; part < 4; part++)
    {
        ULONG byte = 0;
        int digits = 0;
        while (*text >= '0' && *text <= '9' && digits < 3)
        {
            byte = byte * 10u + (ULONG)(*text++ - '0');
            digits++;
        }
        if (digits == 0 || byte > 255u || *text != (part < 3 ? '.' : '\0'))
        {
            return -1;
        }
        text++;
        value = (value << 8) | byte;
    }
    *address = value;
    return 0;
}

/* Logs the presence sensor result when it changes (step name + status). */
static void presence_report(const char* step, UINT status)
{
    static const char* last_step;
    static UINT last_status;
    if (step != last_step || status != last_status)
    {
        printf("Presence sensor: %s (0x%02x)\r\n", step, status);
        last_step   = step;
        last_status = status;
    }
}

/* One HTTP/1.0 exchange; reply (NUL terminated) in `reply`. 0 = ok. */
static int presence_http_get(ULONG server, char* reply, ULONG cap)
{
    static const char request[] = "GET " PRESENCE_SENSOR_PATH " HTTP/1.0\r\n"
                                  "Host: " PRESENCE_SENSOR_HOST "\r\n"
                                  "Connection: close\r\n\r\n";
    ULONG timeout = ms_to_ticks(PRESENCE_TIMEOUT_MS);
    ULONG length  = 0;
    int result    = -1;

    UINT status = nx_tcp_client_socket_bind(&presence_socket, NX_ANY_PORT, timeout);
    if (status != NX_SUCCESS)
    {
        presence_report("bind failed", status);
        return -1;
    }
    status = nx_tcp_client_socket_connect(&presence_socket, server, PRESENCE_PORT, timeout);
    if (status != NX_SUCCESS)
    {
        presence_report("connect failed", status);
    }
    else
    {
        NX_PACKET* packet = packet_from((const uint8_t*)request, sizeof(request) - 1u, NX_TCP_PACKET);
        int sent          = 0;
        if (packet == NX_NULL)
        {
            presence_report("no packet", 0);
        }
        else if ((status = nx_tcp_socket_send(&presence_socket, packet, timeout)) != NX_SUCCESS)
        {
            presence_report("send failed", status);
            nx_packet_release(packet);
        }
        else
        {
            /* (the NetX send macro has cleared `packet`: it is NetX's now) */
            sent = 1;
        }
        if (sent)
        {
            /* Until the server closes the connection (HTTP/1.0). */
            while (nx_tcp_socket_receive(&presence_socket, &packet, timeout) == NX_SUCCESS)
            {
                ULONG copied = 0;
                nx_packet_data_extract_offset(packet, 0, reply + length, cap - 1u - length, &copied);
                length += copied;
                nx_packet_release(packet);
            }
            result = length > 0 ? 0 : -1;
            presence_report(length > 0 ? "ok" : "empty reply", 0);
        }
        nx_tcp_socket_disconnect(&presence_socket, timeout);
    }
    nx_tcp_client_socket_unbind(&presence_socket);
    reply[length] = '\0';
    return result;
}

int32_t plat_presence_read(void)
{
    static char reply[PRESENCE_REPLY_MAX];
    ULONG server;

    if (ip_address == 0)
    {
        return -1;
    }
    if (parse_ipv4(PRESENCE_SENSOR_HOST, &server) != 0)
    {
        presence_report("bad address " PRESENCE_SENSOR_HOST, 0);
        return -1;
    }
    if (!presence_socket_ready)
    {
        UINT status = nx_tcp_socket_create(&nx_ip, &presence_socket, "presence", NX_IP_NORMAL, NX_FRAGMENT_OKAY,
            NX_IP_TIME_TO_LIVE, PRESENCE_REPLY_MAX, NX_NULL, NX_NULL);
        if (status != NX_SUCCESS)
        {
            presence_report("socket create failed", status);
            return -1;
        }
        presence_socket_ready = 1;
    }
    if (presence_http_get(server, reply, sizeof(reply)) != 0 || strncmp(reply, "HTTP/1.", 7) != 0 ||
        strstr(reply, " 200 ") == NULL)
    {
        return -1;
    }
    if (strstr(reply, "\"value\":true") != NULL)
    {
        return 1;
    }
    if (strstr(reply, "\"value\":false") != NULL)
    {
        return 0;
    }
    return -1;
}
