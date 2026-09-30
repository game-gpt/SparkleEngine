//! CPU 参考：把一次纯色填充降为 UCF `Fill` 图并读回。

use std::collections::BTreeMap;

use ucf::prelude::*;
use ucf::SchedulerError;

/// 一帧内可提交的计算 pass 描述（当前仅纯色 Fill）。
#[derive(Debug, Clone)]
pub struct SparkComputePass {
    /// 调试名（不进 `.ucf` wire）。
    pub name: String,
    /// 输出宽（像素）。
    pub width: u32,
    /// 输出高（像素）。
    pub height: u32,
    /// 目标 RGBA8 颜色。
    pub rgba: [u8; 4],
}

impl SparkComputePass {
    /// 构造纯色填充 pass。
    pub fn color_fill(name: impl Into<String>, width: u32, height: u32, rgba: [u8; 4]) -> Self {
        Self {
            name: name.into(),
            width,
            height,
            rgba,
        }
    }

    fn pixel_count(&self) -> usize {
        (self.width as usize).saturating_mul(self.height as usize)
    }
}

/// 持有 `CpuSession` 的薄执行器（与 `SparkRuntime` 世界状态无关）。
pub struct SparkUcfExecutor {
    session: CpuSession,
}

impl SparkUcfExecutor {
    /// 打开 CPU UCF 会话。
    pub fn open_cpu() -> Self {
        Self {
            session: CpuSession::open(),
        }
    }

    /// 执行纯色 Fill：UCF `Fill` 写入灰度 `f32` 缓冲并校验，再展开为 RGBA8 供纹理导入。
    ///
    /// 这是应用契约切片，不是最终 GPU 路径；用于验证图结构与 readback。
    pub fn run_color_fill(&mut self, pass: &SparkComputePass) -> Result<Vec<u8>, SchedulerError> {
        self.compute_color_fill_rgba8(pass)
    }

    /// 同 [`run_color_fill`]，供 wgpu / DrawList 桥接复用。
    pub fn compute_color_fill_rgba8(
        &mut self,
        pass: &SparkComputePass,
    ) -> Result<Vec<u8>, SchedulerError> {
        let pixels = pass.pixel_count();
        if pixels == 0 {
            return Ok(Vec::new());
        }
        let gray = f64::from(pass.rgba[0]) / 255.0;
        let graph = fill_graph(pixels, gray);
        self.session.clear_diagnostics();
        self.session.prepare(&graph)?;
        self.session.run_prepared(&graph)?;
        self.session.flush()?;
        let floats = self.session.read_f32(ResourceId(1))?;
        if floats.len() != pixels {
            return Err(SchedulerError::Backend(
                "cpu".into(),
                format!(
                    "spark-ucf-adapter fill expected {pixels} f32, got {}",
                    floats.len()
                ),
            ));
        }
        let expect = pass.rgba[0] as f32 / 255.0;
        if floats.iter().any(|v| (v - expect).abs() > 1e-5) {
            return Err(SchedulerError::Backend(
                "cpu".into(),
                "spark-ucf-adapter fill readback mismatch".into(),
            ));
        }
        let mut out = Vec::with_capacity(pixels * 4);
        for _ in 0..pixels {
            out.extend_from_slice(&pass.rgba);
        }
        Ok(out)
    }

    /// UCF CPU Fill → staging 上传到 [`WgpuRgba8Target`]（非 device-resident UCF）。
    #[cfg(feature = "wgpu")]
    pub fn run_color_fill_into_wgpu(
        &mut self,
        pass: &SparkComputePass,
        queue: &wgpu::Queue,
        target: &crate::wgpu_target::WgpuRgba8Target,
    ) -> Result<(), SchedulerError> {
        if target.width() != pass.width || target.height() != pass.height {
            return Err(SchedulerError::Backend(
                "wgpu".into(),
                format!(
                    "target {}x{} does not match pass {}x{}",
                    target.width(),
                    target.height(),
                    pass.width,
                    pass.height
                ),
            ));
        }
        let rgba = self.compute_color_fill_rgba8(pass)?;
        crate::wgpu_target::WgpuRgba8Target::upload_rgba8(queue, target, &rgba).map_err(|e| {
            SchedulerError::Backend("wgpu".into(), e)
        })
    }

    /// 能力探测（应含 `cpu`）。
    pub fn capabilities(&self) -> CapabilityReport {
        self.session.capabilities()
    }

    /// 最近一次执行的诊断事件。
    pub fn diagnostics(&self) -> &ExecutionDiagnostics {
        self.session.diagnostics()
    }
}

fn fill_graph(floats: usize, value: f64) -> Graph {
    Graph {
        resources: ResourceGraph {
            nodes: vec![ResourceNode {
                id: ResourceId(1),
                kind: ResourceKind::Buffer,
                domain: Domain::Host,
                access: Access::ReadWrite,
                byte_size: Some((floats * 4) as u64),
            }],
        },
        tasks: TaskGraph {
            nodes: vec![TaskNode {
                id: TaskId(10),
                kind: TaskKind::Fill,
                shader: ShaderId(10),
                params: BTreeMap::from([
                    ("dst".into(), ParamValue::I64(1)),
                    ("value".into(), ParamValue::F64(value)),
                ]),
                dispatch: Default::default(),
                objective: Objective::MaxThroughput,
                priority: Priority::Batch,
            }],
            edges: vec![DepEdge {
                from_task: None,
                from_resource: Some(ResourceId(1)),
                to_task: TaskId(10),
                kind: DepKind::Data,
            }],
        },
    }
}
