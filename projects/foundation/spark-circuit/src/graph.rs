//! 无向导体图与按通道的可达查询。

use std::fmt;

use spark_types::{ErrorArg, ErrorArgs};

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

/// `POWER` 通道上的额定聚合（无潮流求解）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerBudget {
    /// 可达电源额定之和。
    pub supply: f32,
    /// 可达负载需求之和。
    pub demand: f32,
}

impl PowerBudget {
    /// 供电是否覆盖需求（含 `1e-5` 浮点容差）。
    pub fn satisfied(self) -> bool {
        self.supply + 1e-5 >= self.demand
    }

    /// 富余供电（不足时为 0）。
    pub fn surplus(self) -> f32 {
        (self.supply - self.demand).max(0.0)
    }

    /// 缺口需求（有富余时为 0）。
    pub fn deficit(self) -> f32 {
        (self.demand - self.supply).max(0.0)
    }
}

/// 电路图错误。`Display` 只输出稳定码。
#[derive(Debug, PartialEq, Eq)]
pub enum CircuitError {
    /// 引用了未分配的节点 id。
    UnknownNode(NodeId),
}

impl CircuitError {
    /// 稳定错误码（`spark.circuit.*`）。
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownNode(_) => "spark.circuit.unknown_node",
        }
    }

    /// 类型化参数（如 `node` 下标）。
    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::UnknownNode(id) => ErrorArgs::new().with("node", ErrorArg::Unsigned(u64::from(*id))),
        }
    }
}

impl fmt::Display for CircuitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for CircuitError {}

/// 多通道无向图。边变更后可惰性或立即重算可达。
#[derive(Debug, Clone, Default)]
pub struct CircuitGraph {
    node_count: u32,
    /// `adj[channel][node] = 邻居列表`
    adj: Vec<Vec<Vec<NodeId>>>,
    dirty: bool,
    /// 节点供电额定（>0 视为源）。长度随节点增长。
    power_supply: Vec<f32>,
    /// 节点用电需求（>0 视为汇）。
    power_demand: Vec<f32>,
}

impl CircuitGraph {
    /// 空图。
    pub fn new() -> Self {
        Self::default()
    }

    /// 已分配节点数。
    pub fn node_count(&self) -> u32 {
        self.node_count
    }

    /// 边或电源角色自上次清除后是否变更（调用方决定何时重算）。
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 手动标脏（例如外部批量改图后）。
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 清除脏标记（不自动触发重算）。
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
        self.power_supply.push(0.0);
        self.power_demand.push(0.0);
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
        let Some(adj) = self.adj.get_mut(channel.0 as usize)
        else {
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
        self.power_supply.clear();
        self.power_demand.clear();
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
    pub fn reachable_from_any(&self, sources: &[NodeId], channel: Channel) -> Result<Vec<bool>, CircuitError> {
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
                let Some(neis) = adj.get(cur as usize)
                else {
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
    pub fn same_component(&self, a: NodeId, b: NodeId, channel: Channel) -> Result<bool, CircuitError> {
        if a >= self.node_count {
            return Err(CircuitError::UnknownNode(a));
        }
        if b >= self.node_count {
            return Err(CircuitError::UnknownNode(b));
        }
        let labels = self.connected_components(channel);
        Ok(labels[a as usize] == labels[b as usize])
    }

    /// 将节点标为电源（额定 `supply`，≤0 清除源角色）。不改需求。
    pub fn set_power_source(&mut self, node: NodeId, supply: f32) -> Result<(), CircuitError> {
        if node >= self.node_count {
            return Err(CircuitError::UnknownNode(node));
        }
        self.power_supply[node as usize] = supply.max(0.0);
        self.dirty = true;
        Ok(())
    }

    /// 将节点标为负载（需求 `demand`，≤0 清除汇角色）。不改供电。
    pub fn set_power_sink(&mut self, node: NodeId, demand: f32) -> Result<(), CircuitError> {
        if node >= self.node_count {
            return Err(CircuitError::UnknownNode(node));
        }
        self.power_demand[node as usize] = demand.max(0.0);
        self.dirty = true;
        Ok(())
    }

    /// 清除节点的供电与需求角色。
    pub fn clear_power_role(&mut self, node: NodeId) -> Result<(), CircuitError> {
        if node >= self.node_count {
            return Err(CircuitError::UnknownNode(node));
        }
        self.power_supply[node as usize] = 0.0;
        self.power_demand[node as usize] = 0.0;
        self.dirty = true;
        Ok(())
    }

    /// 读取节点供电额定。
    pub fn power_supply_of(&self, node: NodeId) -> Result<f32, CircuitError> {
        if node >= self.node_count {
            return Err(CircuitError::UnknownNode(node));
        }
        Ok(self.power_supply[node as usize])
    }

    /// 读取节点用电需求。
    pub fn power_demand_of(&self, node: NodeId) -> Result<f32, CircuitError> {
        if node >= self.node_count {
            return Err(CircuitError::UnknownNode(node));
        }
        Ok(self.power_demand[node as usize])
    }

    /// 从若干电源出发，在 `POWER` 通道上汇总可达供电与负载。
    ///
    /// 空源：supply/demand 均为 0。不做潮流分配，只做额定聚合。
    pub fn power_budget(&self, sources: &[NodeId]) -> Result<PowerBudget, CircuitError> {
        for &s in sources {
            if s >= self.node_count {
                return Err(CircuitError::UnknownNode(s));
            }
        }
        let mask = self.bfs(sources, Channel::POWER);
        let mut supply = 0.0f32;
        let mut demand = 0.0f32;
        for (i, &reach) in mask.iter().enumerate() {
            if !reach {
                continue;
            }
            supply += self.power_supply[i];
            demand += self.power_demand[i];
        }
        Ok(PowerBudget { supply, demand })
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
            let Some(neis) = adj.get(cur as usize)
            else {
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
