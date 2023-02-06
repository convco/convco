pub trait Strip {
    fn strip(&self) -> String;
}

impl<T> Strip for T
where
    T: AsRef<str>,
{
    fn strip(&self) -> String {
        fn strip(s: &str) -> String {
            let iter = s
                .lines()
                .filter(|line| !line.starts_with('#'))
                .map(|line| line.trim_end());
            let mut lines: Vec<&str> = iter.collect();
            lines.dedup_by(|a, b| a.trim().is_empty() && b.trim().is_empty());
            lines.join("\n").trim().to_string()
        }
        strip(self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let s = "#


1
2  
3 


4
# 5

";
        assert_eq!("1\n2\n3\n\n4", s.strip());
    }
}
