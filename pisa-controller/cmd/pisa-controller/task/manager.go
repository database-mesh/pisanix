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

package task

import "golang.org/x/sync/errgroup"

type TaskManager struct {
	//TODO: consider to be a FIFO queue
	TaskGroups []Task
	eg         errgroup.Group
}

type Task interface {
	Run() error
}

func (m *TaskManager) Register(t Task) *TaskManager {
	m.TaskGroups = append(m.TaskGroups, t)
	return m
}

func (m *TaskManager) Run() error {
	for idx := range m.TaskGroups {
		m.eg.Go(m.TaskGroups[idx].Run)
	}
	return m.eg.Wait()
}
