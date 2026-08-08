//! 响应压缩：按 `Accept-Encoding` 自动选择 gzip/br/zstd。
//! 使用 tower-http 内置的 [`CompressionLayer`]。

use tower_http::compression::CompressionLayer;

pub fn layer() -> CompressionLayer {
    CompressionLayer::new()
}
