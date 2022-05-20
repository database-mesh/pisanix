# Pisa-Daemon 

Pisa-Daemon provides QoS for traffic in a cloud native way.

## Features

Different applications in production clusters are always applied with different priorities. For better SLA purpose, we treat application as different QoS class. At present, Kubernetes provides CPU and Memory QoS, and community has contributed some design for network QoS, such as ingress and egress traffic bandwidth. Pisa-Daemon could provider a protocol-specific network QoS solution with the help of Traffic Control and eBPF.