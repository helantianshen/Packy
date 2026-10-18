package domain

import (
	"packy/pkg/protocol"
	"sort"
)

// Diff 按稳定 Mod ID 比较完整快照，不读取玩家目录
func Diff(old, new []protocol.ReleaseItem) []protocol.Operation {
	before := map[string]protocol.ReleaseItem{}
	after := map[string]protocol.ReleaseItem{}
	for _, v := range old {
		before[v.ModID] = v
	}
	for _, v := range new {
		after[v.ModID] = v
	}
	out := []protocol.Operation{}
	for id, v := range after {
		previous, exists := before[id]
		target := v.Target
		if !exists {
			out = append(out, protocol.Operation{Action: "ADD", ModID: id, Target: &target})
		} else if previous.Target.Path != v.Target.Path || previous.Target.SHA512 != v.Target.SHA512 {
			out = append(out, protocol.Operation{Action: "REPLACE", ModID: id, OldPath: previous.Target.Path, Target: &target})
		}
	}
	for id, v := range before {
		if _, exists := after[id]; !exists {
			out = append(out, protocol.Operation{Action: "REMOVE", ModID: id, OldPath: v.Target.Path})
		}
	}
	sort.Slice(out, func(i, j int) bool { return out[i].ModID < out[j].ModID })
	return out
}
