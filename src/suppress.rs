// suppress.rs — inline 抑制（biome-ignore と同じ役割分担）。
// correo.toml の allow は語彙の恒久裁定、こちらは一回きりの言及をその場で逃がす手段:
//   <!-- correo-ignore -->                    直後の行（と同じ行）の全指摘を抑制
//   <!-- correo-ignore no-chain coinage -->   規則名 / 検出器名を絞って抑制
// 「correo-ignore」を含む行なら書式は問わない（markdown コメント以外の文脈でも効く）。
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct Suppressions {
    /// 行番号（1 始まり）→ 抑制対象。None = 全規則。
    by_line: HashMap<usize, Option<HashSet<String>>>,
}

pub fn scan(text: &str) -> Suppressions {
    let mut s = Suppressions::default();
    for (i, line) in text.lines().enumerate() {
        let Some(rest) = line.split("correo-ignore").nth(1) else {
            continue;
        };
        let rest = rest.trim().trim_end_matches("-->").trim();
        let names: HashSet<String> = rest
            .split([' ', ',', ':'])
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect();
        let entry = if names.is_empty() { None } else { Some(names) };
        for l in [i + 1, i + 2] {
            // 1 始まりで「同じ行」= i+1、「次の行」= i+2
            merge(&mut s.by_line, l, entry.clone());
        }
    }
    s
}

fn merge(m: &mut HashMap<usize, Option<HashSet<String>>>, line: usize, e: Option<HashSet<String>>) {
    use std::collections::hash_map::Entry;
    match m.entry(line) {
        Entry::Vacant(v) => {
            v.insert(e);
        }
        Entry::Occupied(mut o) => {
            let slot = o.get_mut();
            match e {
                None => *slot = None, // 全抑制が最強
                Some(new) => {
                    if let Some(set) = slot.as_mut() {
                        set.extend(new);
                    } // slot が None（全抑制）なら何もしない
                }
            }
        }
    }
}

impl Suppressions {
    pub fn hit(&self, line: usize, detector: &str, rule: &str) -> bool {
        match self.by_line.get(&line) {
            None => false,
            Some(None) => true,
            Some(Some(set)) => set.contains(detector) || set.contains(rule),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_ignore_suppresses_next_line_for_all_rules() {
        let s = scan("<!-- correo-ignore -->\nこの行の指摘は全部消える。\n三行目は消えない。");
        assert!(s.hit(2, "readability", "no-chain"));
        assert!(s.hit(2, "coinage", "dictionary-coinage"));
        assert!(!s.hit(3, "readability", "no-chain"));
    }

    #[test]
    fn named_ignore_suppresses_only_named_rule_or_detector() {
        let s = scan("本文 <!-- correo-ignore no-chain coinage -->");
        assert!(s.hit(1, "readability", "no-chain"), "規則名で抑制");
        assert!(s.hit(1, "coinage", "dictionary-coinage"), "検出器名で抑制");
        assert!(
            !s.hit(1, "readability", "sentence-length"),
            "無関係の規則は生きる"
        );
    }
}
