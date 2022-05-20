// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package bpf

import (
	"bytes"
	"encoding/binary"
	"fmt"

	// "C"
	"net"
	"syscall"

	"github.com/cilium/ebpf"
	"github.com/cilium/ebpf/ringbuf"
	"github.com/mlycore/log"
	"golang.org/x/sys/unix"
)

const (
	SockFilter = "sock_filter.o"
	TcPktMap   = "/sys/fs/bpf/tc/globals/my_pkt"
)

type Loader struct {
}

func (l *Loader) Load(ifaceName string, port uint16) error {
	tcMap, err := l.LoadTcPkgMap()
	// TODO: add loader to load this program to net dev
	// TODO: add port

	if err != nil {
		return err
	}

	if err := l.LoadSockFilter(ifaceName, port, tcMap); err != nil {
		return err
	}

	return nil
}

type Objs struct {
	Prog         *ebpf.Program `ebpf:"sql_filter"`
	TailProg1    *ebpf.Program `ebpf:"sql_filter_1"`
	FilterHelper *ebpf.Map     `ebpf:"filter_helper"`
	Buf          *ebpf.Map     `ebpf:"buf"`
	JmpTable     *ebpf.Map     `ebpf:"jmp_table"`
	MyPkt        *ebpf.Map     `ebpf:"my_pkt"`
	MyPktEvt     *ebpf.Map     `ebpf:"my_pkt_evt"`
}

func (l *Loader) LoadSockFilter(ifaceName string, port uint16, tcPkt *ebpf.Map) error {
	spec, err := ebpf.LoadCollectionSpec(SockFilter)
	if err != nil {
		return err
	}

	var objs Objs
	if err := spec.LoadAndAssign(&objs, nil); err != nil {
		return err
	}

	if err = objs.JmpTable.Update(uint32(0), uint32(objs.TailProg1.FD()), ebpf.UpdateAny); err != nil {
		return fmt.Errorf("jmptable err: %s", err)
	}

	if err = objs.FilterHelper.Update(uint8(0), port, ebpf.UpdateAny); err != nil {
		return fmt.Errorf("set filter port error: %v", err)
	}

	sock, err := l.OpenRawSock(ifaceName)
	if err != nil {
		return err
	}

	defer syscall.Close(sock)

	if err := syscall.SetsockoptInt(sock, syscall.SOL_SOCKET, unix.SO_ATTACH_BPF, objs.Prog.FD()); err != nil {
		return err
	}

	defer objs.Prog.Close()

	reader, err := ringbuf.NewReader(objs.MyPktEvt)
	if err != nil {
		return err
	}

	for {
		evt, query, err := l.ReadRecord(objs, reader)
		if err != nil {
			log.Warnln(err)
			continue
		}

		evt_value := EventValue{
			ClassId: l.CalcQoS(query),
		}

		if err := tcPkt.Update(&evt, &evt_value, ebpf.UpdateAny); err != nil {
			log.Warnln(err)
		}
	}
	return nil
}

func (l *Loader) OpenRawSock(ifaceName string) (int, error) {
	sock, err := syscall.Socket(syscall.AF_PACKET, syscall.SOCK_RAW|syscall.SOCK_NONBLOCK|syscall.SOCK_CLOEXEC,
		int(htons(syscall.ETH_P_ALL)))
	if err != nil {
		return -1, err
	}

	iface, err := net.InterfaceByName(ifaceName)
	if err != nil {
		return -1, err
	}

	sll := syscall.SockaddrLinklayer{
		Ifindex:  iface.Index,
		Protocol: htons(syscall.ETH_P_ALL),
	}

	if err := syscall.Bind(sock, &sll); err != nil {
		return -1, err
	}

	return sock, nil
}

func (l *Loader) LoadTcPkgMap() (*ebpf.Map, error) {
	return ebpf.LoadPinnedMap(TcPktMap, nil)
}
func (l *Loader) CalcQoS(query string) uint32 {
	//TODO: virtual database tc argument
	return 0
}

type EventKey struct {
	Seq   uint8
	Sport uint16
	Dport uint16
	Saddr uint32
	Daddr uint32
}

type EventValue struct {
	// tcp payload offset
	Offset  uint32
	PktLen  uint32
	ClassId uint32
}

// readRecord read record from ringbuf
func (l *Loader) ReadRecord(objs Objs, reader *ringbuf.Reader) (EventKey, string, error) {
	record, err := reader.Read()
	if err != nil {
		return EventKey{}, "", fmt.Errorf("reading from reader: %s", err)
	}

	var evt EventKey
	if err := binary.Read(bytes.NewBuffer(record.RawSample), binary.LittleEndian, &evt); err != nil {
		return EventKey{}, "", fmt.Errorf("parsing ringbuf event: %s", err)
	}

	var evt_value EventValue
	if err := objs.MyPkt.Lookup(&evt, &evt_value); err != nil {
		return EventKey{}, "", fmt.Errorf("read event value: %s", err)
	}

	var (
		key     uint32
		value   uint8
		entries = objs.Buf.Iterate()
		chars   = make([]byte, evt_value.PktLen)
	)

	// PktLen contains COM_TYPE 1byte, so evt.PktLen-1 here
	for i := 0; i < int(evt_value.PktLen-1); i++ {
		entries.Next(&key, &value)
		chars = append(chars, value)
	}

	return evt, string(chars), nil
}
