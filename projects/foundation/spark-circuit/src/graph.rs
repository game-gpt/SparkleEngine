//! 无向导体图与按通道的可达查询。

use thiserror::Error;

/// 图内节点句柄（稠密从 0 递增）。
pub type NodeId = u32;

/// 信号 / 能量通道。框架不解释含义，由调用方约定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Channel(pub u8);

impl Channel {
    /// 控制通道（约定俗成 0）。
    pub const CONTROL: Self = Self(0);
    /// 电力通道（约定俗成 1）。
    pub const POWER: Self = Self(1);
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CircuitError {
    #[error("未知节点：{0}")]
    UnknownNode(NodeId),
}

/// 多通道无向图。边变更后可惰性或立即重算可达。
#[derive(Debug, Clone, Default)]
pub struct CircuitGraph {
    node_count: u32,
    /// `adj[channel][node] = 邻居列表`
    adj: Vec<Vec<Vec<NodeId>>>,
    dirty: bool,
}

impl CircuitGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_count(&self) -> u32 {
        self.node_count
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// 分配新节点，返回其 id。
    pub fn add_node(&mut self) -> NodeId {
        let id = self.node_count;
        self.node_count += 1;
        for ch in &mut self.adj {
            ch.push(Vec::new());
        }
        self.dirty = true;
        id
    }

    fn ensure_channel(&mut self, channel: Channel) {
        let idx = channel.0 as usize;
        while self.adj.len() <= idx {
            self.adj.push(vec![Vec::new(); self.node_count as usize]);
        }
    }

    fn channel_adj(&self, channel: Channel) -> Option<&Vec<Vec<NodeId>>> {
        self.adj.get(channel.0 as usize)
    }

    /// 在通道上添加无向边。重复边忽略。
    pub fn link(&mut self, a: NodeId, b: NodeId, channel: Channel) -> Result<(), CircuitError> {
        if a >= self.node_count || b >= self.node_count {
            return Err(CircuitError::UnknownNode(a.max(b)));
        }
        if a == b {
            return Ok(());
        }
        self.ensure_channel(channel);
        let idx = channel.0 as usize;
        let adj = &mut self.adj[idx];
        if !adj[a as usize].contains(&b) {
            adj[a as usize].push(b);
        }
        if !adj[b as usize].contains(&a) {
            adj[b as usize].push(a);
        }
        self.dirty = true;
        Ok(())
    }

    /// 移除通道上的无向边（若存在）。
    pub fn unlink(&mut self, a: NodeId, b: NodeId, channel: Channel) -> Result<(), CircuitError> {
        if a >= self.node_count || b >= self.node_count {
            return Err(CircuitError::UnknownNode(a.max(b)));
        }
        let Some(adj) = self.adj.get_mut(channel.0 as usize) else {
            return Ok(());
        };
        adj[a as usize].retain(|&n| n != b);
        adj[b as usize].retain(|&n| n != a);
        self.dirty = true;
        Ok(())
    }

    /// 清空全部节点与边。
    pub fn clear(&mut self) {
        self.node_count = 0;
        self.adj.clear();
        self.dirty = true;
    }

    /// 单源可达（含起点）。未知节点返回错误。
    pub fn reachable(&self, from: NodeId, channel: Channel) -> Result<Vec<bool>, CircuitError> {
        if from >= self.node_count {
            return Err(CircuitError::UnknownNode(from));
        }
        Ok(self.bfs(&[from], channel))
    }

    /// 多源可达（含所有源）。空源得到全 false。
    pub fn reachable_from_any(
        &self,
        sources: &[NodeId],
        channel: Channel,
    ) -> Result<Vec<bool>, CircuitError> {
        for &s in sources {
            if s >= self.node_count {
                return Err(CircuitError::UnknownNode(s));
            }
        }
        Ok(self.bfs(sources, channel))
    }

    /// `from` 能否沿通道到达 `to`。
    pub fn can_reach(&self, from: NodeId, to: NodeId, channel: Channel) -> Result<bool, CircuitError> {
        let mask = self.reachable(from, channel)?;
        if to >= self.node_count {
            return Err(CircuitError::UnknownNode(to));
        }
        Ok(mask[to as usize])
    }

    /// 按通道计算连通分量。返回与节点数等长的分量 id（从 0 递增，孤立点各成一分量）。
    ///
    /// 空图返回空向量。`dirty` 标记不会自动清除，由调用方决定何时重算。
    pub fn connected_components(&self, channel: Channel) -> Vec<u32> {
        let n = self.node_count as usize;
        if n == 0 {
            return Vec::new();
        }
        let empty: Vec<Vec<NodeId>> = Vec::new();
        let adj = self.channel_adj(channel).unwrap_or(&empty);
        let mut comp = vec![u32::MAX; n];
        let mut next = 0u32;
        let mut stack = Vec::new();
        for start in 0..n {
            if comp[start] != u32::MAX {
                continue;
            }
            let cid = next;
            next += 1;
            comp[start] = cid;
            stack.clear();
            stack.push(start as NodeId);
            while let Some(cur) = stack.pop() {
                let Some(neis) = adj.get(cur as usize) else {
                    continue;
                };
                for &n in neis {
                    let i = n as usize;
                    if comp[i] == u32::MAX {
                        comp[i] = cid;
                        stack.push(n);
                    }
                }
            }
        }
        comp
    }

    /// 将 [`connected_components`] 结果收成「分量 → 节点列表」。
    pub fn component_lists(&self, channel: Channel) -> Vec<Vec<NodeId>> {
        let labels = self.connected_components(channel);
        if labels.is_empty() {
            return Vec::new();
        }
        let count = labels.iter().copied().max().unwrap_or(0) as usize + 1;
        let mut lists = vec![Vec::new(); count];
        for (i, &cid) in labels.iter().enumerate() {
            lists[cid as usize].push(i as NodeId);
        }
        lists
    }

    /// `a` 与 `b` 是否在同一连通分量（同通道）。
    pub fn same_component(
        &self,
        a: NodeId,
        b: NodeId,
        channel: Channel,
    ) -> Result<bool, CircuitError> {
        if a >= self.node_count {
            return Err(CircuitError::UnknownNode(a));
        }
        if b >= self.node_count {
            return Err(CircuitError::UnknownNode(b));
        }
        let labels = self.connected_components(channel);
        Ok(labels[a as usize] == labels[b as usize])
    }

    fn bfs(&self, sources: &[NodeId], channel: Channel) -> Vec<bool> {
        let n = self.node_count as usize;
        let mut seen = vec![false; n];
        if sources.is_empty() || n == 0 {
            return seen;
        }
        let empty: Vec<Vec<NodeId>> = Vec::new();
        let adj = self.channel_adj(channel).unwrap_or(&empty);
        let mut queue = Vec::with_capacity(sources.len());
        for &s in sources {
            let i = s as usize;
            if !seen[i] {
                seen[i] = true;
                queue.push(s);
            }
        }
        let mut qh = 0;
        while qh < queue.len() {
            let cur = queue[qh];
            qh += 1;
            let Some(neis) = adj.get(cur as usize) else {
                continue;
            };
            for &n in neis {
                let i = n as usize;
                if !seen[i] {
                    seen[i] = true;
                    queue.push(n);
                }
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_path_and_cut() {
        let mut g = CircuitGraph::new();
        let cockpit = g.add_node();
        let cable = g.add_node();
        let engine_a = g.add_node();
        let engine_b = g.add_node();
        g.link(cockpit, cable, Channel::CONTROL).unwrap();
        g.link(cable, engine_a, Channel::CONTROL).unwrap();
        g.link(cable, engine_b, Channel::CONTROL).unwrap();

        assert!(g.can_reach(cockpit, engine_a, Channel::CONTROL).unwrap());
        assert!(g.can_reach(cockpit, engine_b, Channel::CONTROL).unwrap());

        g.unlink(cable, engine_b, Channel::CONTROL).unwrap();
        assert!(g.can_reach(cockpit, engine_a, Channel::CONTROL).unwrap());
        assert!(!g.can_reach(cockpit, engine_b, Channel::CONTROL).unwrap());
    }

    #[test]
    fn multi_source_reachability() {
        let mut g = CircuitGraph::new();
        let a = g.add_node();
        let b = g.add_node();
        let c = g.add_node();
        g.link(a, b, Channel::POWER).unwrap();
        let mask = g.reachable_from_any(&[a, c], Channel::POWER).unwrap();
        assert!(mask[a as usize]);
        assert!(mask[b as usize]);
        assert!(mask[c as usize]);
    }

    #[test]
    fn connected_components_split_on_cut() {
        let mut g = CircuitGraph::new();
        let a = g.add_node();
        let b = g.add_node();
        let c = g.add_node();
        g.link(a, b, Channel::CONTROL).unwrap();
        g.link(b, c, Channel::CONTROL).unwrap();
        assert_eq!(g.connected_components(Channel::CONTROL).iter().max(), Some(&0));
        assert!(g.same_component(a, c, Channel::CONTROL).unwrap());

        g.unlink(b, c, Channel::CONTROL).unwrap();
        let lists = g.component_lists(Channel::CONTROL);
        assert_eq!(lists.len(), 2);
        assert!(!g.same_component(a, c, Channel::CONTROL).unwrap());
    }
}
