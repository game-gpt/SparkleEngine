//! Spark GPU 友好纹理模型：描述、数据、上传包、采样器与图集区域。
//!
//! **不**解码源图片文件，**不**依赖 `image` / `wgpu`。
//! 格式插件产出 [`TextureUpload`]；渲染后端消费后得到 GPU 句柄。

#![warn(missing_docs)]

mod atlas;
mod data;
mod desc;
mod format;
mod sampler;
mod upload;
mod usage;

pub use atlas::{AtlasMetadata, SpriteRegion};
pub use data::{TextureData, TextureLayout};
pub use desc::{TextureDesc, TextureInfo};
pub use format::{AlphaMode, ColorSpace, TextureDimension, TextureFormat};
pub use sampler::{AddressMode, FilterMode, SamplerDesc, TextureState};
pub use upload::{CpuCopyPolicy, MipmapPolicy, Residency, TextureUpload, UploadPolicy};
pub use usage::{DeviceCaps, TextureUsage};
