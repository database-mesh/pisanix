/**
 * Copyright 2022 SphereEx Authors
 * 
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * 
 *     http://www.apache.org/licenses/LICENSE-2.0
 * 
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include <linux/bpf.h>
#include <linux/ip.h>
#include <linux/tcp.h>
#include <arpa/inet.h>
#include <linux/if_ether.h>
#include "headers/bpf_helpers.h"
#include "headers/bpf_endian.h"

struct endpoint {
	__u32 ip;
    __u16 port;
};

struct bpf_map_def SEC("maps") app_endpoints = {
    .type = BPF_MAP_TYPE_HASH,
    .key_size = sizeof(struct endpoint),
    .value_size = sizeof(char[128]),
    .max_entries = 4096,
    .map_flags = BPF_F_NO_PREALLOC,
};

// Get egress endpoint for matching the DatabaseEndpoint associated with the VirtualDatabase.
SEC("socket/app")
int app_filter(struct __sk_buff *skb) {
    if (skb->protocol != bpf_htons(ETH_P_IP)) {
        return 0;
    }

    struct iphdr iph;
    bpf_skb_load_bytes(skb, ETH_HLEN, &iph, sizeof(iph));

    if (iph.protocol != IPPROTO_TCP) {
    	return 0;
    }

    __u32 daddr = bpf_ntohl(iph.daddr);

    struct tcphdr tcph;

    bpf_skb_load_bytes(skb, ETH_HLEN + sizeof(iph), &tcph, sizeof(tcph));

    __u16 dest_port = bpf_ntohs(tcph.dest);

    struct endpoint ep;
	__builtin_memset(&ep, 0, sizeof(struct endpoint));
    ep.ip = daddr;
    ep.port = dest_port;

    void *app = bpf_map_lookup_elem(&app_endpoints, &ep);

    return app == NULL ? 0 : -1;
}

char LICENSE[] SEC("license") = "GPL";