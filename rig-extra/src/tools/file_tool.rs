//! 文件读取工具
//! todo:: 待完成....

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};
use thiserror::Error;

pub struct FileTool {
    /// 文件安全目录，只运行处理该目录文件
    /// todo:: 变成数组
    safe_dir: String,
}

#[derive(Error, Debug)]
pub enum PathSandboxError {
    #[error("path is outside safe root")]
    OutsideSandbox,

    #[error("path not found: {0}")]
    NotFound(String),

    #[error(transparent)]
    Io(#[from] io::Error),
}

impl FileTool {
    /// 读取 [start, end] 行（从 1 开始）
    fn read_lines_range<P: AsRef<Path>>(
        &self,
        path: P,
        start: usize,
        end: usize,
    ) -> io::Result<Vec<String>> {
        if start == 0 || end < start {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid line range",
            ));
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut result = Vec::new();

        for (idx, line) in reader.lines().enumerate() {
            let line_no = idx + 1;
            if line_no >= start && line_no <= end {
                result.push(line?);
            }
            if line_no > end {
                break;
            }
        }

        Ok(result)
    }

    /// 加上安全沙箱的覆盖函数
    pub fn read_lines_range_safe(
        &self,
        user_path: &str,
        start: usize,
        end: usize,
    ) -> Result<Vec<String>, PathSandboxError> {
        let real_path = self.resolve_and_check(user_path)?;

        self.read_lines_range(real_path, start, end)
            .map_err(PathSandboxError::from)
    }

    /// 用 new_lines 覆盖 [start, end] 行（从 1 开始）
    pub fn stream_replace_range<P: AsRef<Path>>(
        &self,
        path: P,
        start: usize,
        end: usize,
        new_lines: &[String],
    ) -> io::Result<()> {
        let path = path.as_ref();
        let tmp_path = path.with_extension("tmp");

        let input = File::open(path)?;
        let reader = BufReader::new(input);

        let output = File::create(&tmp_path)?;
        let mut writer = BufWriter::new(output);

        let mut line_no = 0;
        let mut replaced = false;

        for line in reader.lines() {
            line_no += 1;
            let line = line?;

            if line_no == start {
                // 写入新内容
                for nl in new_lines {
                    writeln!(writer, "{nl}")?;
                }
                replaced = true;
            }

            if line_no < start || line_no > end {
                writeln!(writer, "{line}")?;
            }
        }

        // 如果原文件行数不足
        if !replaced {
            while line_no < start - 1 {
                writeln!(writer)?;
                line_no += 1;
            }
            for nl in new_lines {
                writeln!(writer, "{nl}")?;
            }
        }

        writer.flush()?;
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    /// 加上安全沙箱的覆盖函数
    pub fn stream_replace_range_safe(
        &self,
        user_path: &str,
        start: usize,
        end: usize,
        new_lines: &[String],
    ) -> Result<(), PathSandboxError> {
        let real_path = self.resolve_and_check(user_path)?;

        self.stream_replace_range(real_path, start, end, new_lines)
            .map_err(PathSandboxError::from)
    }

    /// 插入 不换行
    pub fn stream_insert_at<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        at_line: usize,
        new_lines: &[String],
    ) -> std::io::Result<()> {
        use std::fs::{self, File};
        use std::io::{BufRead, BufReader, BufWriter, Write};

        let path = path.as_ref();
        let tmp_path = path.with_extension("tmp");

        let input = File::open(path)?;
        let reader = BufReader::new(input);

        let output = File::create(&tmp_path)?;
        let mut writer = BufWriter::new(output);

        let mut line_no = 0;
        let mut inserted = false;

        for text in reader.lines() {
            line_no += 1;
            let text = text?;

            if line_no == at_line {
                for nl in new_lines {
                    writeln!(writer, "{nl}")?;
                }
                inserted = true;
            }

            writeln!(writer, "{text}")?;
        }

        // 如果插入点在文件尾之后
        if !inserted {
            for nl in new_lines {
                writeln!(writer, "{nl}")?;
            }
        }

        writer.flush()?;
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    /// 安全插入
    pub fn stream_insert_at_safe(
        &self,
        user_path: &str,
        at_line: usize,
        new_lines: &[String],
    ) -> Result<(), PathSandboxError> {
        let real_path = self.resolve_and_check(user_path)?;

        self.stream_insert_at(real_path, at_line, new_lines)
            .map_err(PathSandboxError::from)
    }

    /// 将用户传入路径解析到真实路径，并校验是否在 safe_root 内
    pub fn resolve_and_check(&self, user_path: &str) -> Result<PathBuf, PathSandboxError> {
        let safe_root = fs::canonicalize(&self.safe_dir)?;

        let user = Path::new(user_path);

        // 1. 禁止绝对路径
        if user.is_absolute() {
            return Err(PathSandboxError::OutsideSandbox);
        }

        // 2. 禁止 .. 路径
        if user
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(PathSandboxError::OutsideSandbox);
        }

        // 3. 禁止以 safe_root 名称或 ./safe_root 名称开头
        if let Some(root_name) = safe_root.file_name() {
            if user.starts_with(root_name) {
                return Err(PathSandboxError::OutsideSandbox);
            }
            if user.starts_with(format!("./{}", root_name.to_string_lossy())) {
                return Err(PathSandboxError::OutsideSandbox);
            }
        }

        // 4. 构造完整路径并解析为绝对路径
        let full = safe_root.join(user);
        let real = fs::canonicalize(&full).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                PathSandboxError::NotFound(full.display().to_string())
            } else {
                PathSandboxError::Io(e)
            }
        })?;

        // 5. 检查解析后的路径是否仍在 safe_root 范围内
        if !real.starts_with(&safe_root) {
            return Err(PathSandboxError::OutsideSandbox);
        }

        Ok(real)
    }
}
