/// 截取字符串前 n 个字符，返回原字符串切片（安全处理UTF-8多字节字符）
pub fn char_prefix(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// 截取字符串末尾 n 个字符，返回原字符串切片（安全处理UTF-8多字节字符）
pub fn char_suffix(s: &str, n: usize) -> &str {
    if n == 0 {
        return "";
    }
    let mut count = 0;
    for (idx, _) in s.char_indices().rev() {
        count += 1;
        if count == n {
            return &s[idx..];
        }
    }
    s // 总字符不足n，返回完整字符串
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 配套工具函数 char_prefix 单元测试
    #[test]
    fn test_char_prefix() {
        // 截取少于总字符
        assert_eq!(char_prefix("rust", 2), "ru");
        // 截取等于总字符
        assert_eq!(char_prefix("rust", 4), "rust");
        // 截取大于总字符，返回原串
        assert_eq!(char_prefix("rust", 10), "rust");
        // UTF-8中文多字节字符测试
        assert_eq!(char_prefix("我爱Rust", 2), "我爱");
        // 空字符串
        assert_eq!(char_prefix("", 5), "");
    }

    /// 配套工具函数 char_suffix 单元测试
    #[test]
    fn test_char_suffix() {
        // n=0 返回空串
        assert_eq!(char_suffix("test", 0), "");
        // 正常截取末尾字符
        assert_eq!(char_suffix("abcde", 2), "de");
        // 截取超过总长度返回原串
        assert_eq!(char_suffix("hi", 10), "hi");
        // 中文多字节截取
        assert_eq!(char_suffix("一二三四", 2), "三四");
        // 空字符串
        assert_eq!(char_suffix("", 3), "");
    }
}
