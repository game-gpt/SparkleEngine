//! Spark GPU 友好纹理模型：描述、数据、上传包、采样器与图集 / 九宫格几何。
//!
//! **不**解码源图片文件，**不**依赖 `image` / `wgpu`。
//! 格式插件产出 [`TextureUpload`]；渲染后端消费后得到 GPU 句柄。

#![forbid(missing_docs)]

mod atlas;
mod data;
mod desc;
mod format;
mod nine;
mod region;
mod sampler;
mod sprite;
mod upload;
mod usage;

pub use atlas::{AtlasMetadata, SpriteRegion};
pub use data::{TextureData, TextureLayout};
pub use desc::{TextureDesc, TextureInfo};
pub use format::{AlphaMode, ColorSpace, TextureDimension, TextureFormat};
pub use nine::{Margin, NineQuad, NineSlice, NineSliceMode};
pub use sampler::{AddressMode, FilterMode, SamplerDesc, TextureState};
pub use sprite::{Sprite, SpriteSheet};
pub use upload::{CpuCopyPolicy, MipmapPolicy, Residency, TextureUpload, UploadPolicy};
pub use usage::{DeviceCaps, TextureUsage};
