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
#include <linux/pkt_cls.h>
#include <linux/ip.h>
#include <linux/tcp.h>
#include <arpa/inet.h>
#include <linux/if_ether.h>
#include "headers/bpf_helpers.h"
#include "headers/bpf_endian.h"

// bpf_elf map definition from https://github.com/shemminger/iproute2/blob/main/include/bpf_elf.h
struct bpf_elf_map {
    unsigned int type;
    unsigned int size_key;
    unsigned int size_value;
    unsigned int max_elem;
    unsigned int flags;
    unsigned int id;
    unsigned int pinning;
    unsigned int inner_id;
    unsigned int inner_idx;
};

struct endpoint {
    __u32 ip;
    __u16 port;
};

struct bpf_elf_map SEC("maps") app_endpoints_classid = {
	.type           = BPF_MAP_TYPE_HASH,
	.size_key       = sizeof(struct endpoint),
	.size_value     = sizeof(__u32),
    	.max_elem       = 4096,
	// Pin path default is /sys/fs/bpf/tc/globals/app-endpoints-classid
	// Pisa-Daemon will write qos rule to my_pkt by pin path
	.pinning        = 2,
};

// attach to eth0 || cni0 || docker0
SEC("classifier")
int tc_egress(struct __sk_buff *skb) {
    if (skb->protocol != bpf_htons(ETH_P_IP)) {
        return TC_ACT_OK;
    }

    struct iphdr iph;
    bpf_skb_load_bytes(skb, ETH_HLEN, &iph, sizeof(iph));

    if (iph.protocol != IPPROTO_TCP) {
    	return TC_ACT_OK;
    }

    struct tcphdr tcph;
    bpf_skb_load_bytes(skb, ETH_HLEN + sizeof(iph), &tcph, sizeof(tcph));

    struct endpoint ep;
    __builtin_memset(&ep, 0, sizeof(struct endpoint));
    
    ep.ip = bpf_ntohl(iph.daddr);
    ep.port = bpf_ntohs(tcph.dest);

    __u32 *class_id;
    class_id = bpf_map_lookup_elem(&app_endpoints_classid, &ep);

    if (class_id) {
    	skb->tc_classid = *class_id;
        return TC_ACT_OK;
    }

    ep.ip = bpf_ntohl(iph.saddr);
    ep.port = bpf_ntohs(tcph.source);

    class_id = bpf_map_lookup_elem(&app_endpoints_classid, &ep);

    if (class_id) {
        skb->tc_classid = *class_id;
    }
    
    return TC_ACT_OK;
}

char LICENSE[] SEC("license") = "GPL";
