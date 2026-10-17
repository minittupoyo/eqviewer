//! JMA2001走時表(気象庁)。深さ方向は線形補間し、震央距離ごとのP/S波到達時間の曲線を返す。
//! 出典: https://www.data.jma.go.jp/svd/eqev/data/bulletin/catalog/appendix/trtime/trt_j.html

use std::sync::OnceLock;

use anyhow::{bail, Result};
use serde::Serialize;

static RAW: &str = include_str!("../data/tjma2001");
static TABLE: OnceLock<Table> = OnceLock::new();

struct Table {
    depths: Vec<f64>,
    dists: Vec<f64>,
    /// depths.len() × dists.len() の行優先
    p: Vec<f64>,
    s: Vec<f64>,
}

/// ある震源の深さにおける、震央距離(km)→到達時間(秒)の曲線
#[derive(Debug, Serialize, PartialEq)]
pub struct Curve {
    pub dist_km: Vec<f64>,
    pub p: Vec<f64>,
    pub s: Vec<f64>,
}

impl Table {
    /// 行形式: `P <p秒> S <s秒> <深さkm> <震央距離km>`(深さ昇順・距離昇順)
    fn parse(text: &str) -> Result<Self> {
        let (mut depths, mut dists, mut p, mut s) = (vec![], vec![], vec![], vec![]);
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let t: Vec<&str> = line.split_whitespace().collect();
            if t.len() != 6 || t[0] != "P" || t[2] != "S" {
                bail!("unexpected line: {line}");
            }
            let (depth, dist): (f64, f64) = (t[4].parse()?, t[5].parse()?);
            if depths.last() != Some(&depth) {
                depths.push(depth);
            }
            if depths.len() == 1 {
                dists.push(dist);
            }
            p.push(t[1].parse()?);
            s.push(t[3].parse()?);
        }
        if p.len() != depths.len() * dists.len() {
            bail!("table is not rectangular");
        }
        Ok(Self { depths, dists, p, s })
    }

    fn row<'a>(&self, values: &'a [f64], i: usize) -> &'a [f64] {
        let n = self.dists.len();
        &values[i * n..(i + 1) * n]
    }

    fn curve(&self, depth_km: f64) -> Curve {
        let d = depth_km.clamp(self.depths[0], *self.depths.last().unwrap());
        let hi = self.depths.partition_point(|x| *x < d).max(1);
        let lo = hi - 1;
        let w = (d - self.depths[lo]) / (self.depths[hi] - self.depths[lo]);
        let lerp = |v: &[f64]| -> Vec<f64> {
            self.row(v, lo)
                .iter()
                .zip(self.row(v, hi))
                .map(|(a, b)| a + (b - a) * w)
                .collect()
        };
        Curve {
            dist_km: self.dists.clone(),
            p: lerp(&self.p),
            s: lerp(&self.s),
        }
    }
}

pub fn curve(depth_km: f64) -> Curve {
    TABLE
        .get_or_init(|| Table::parse(RAW).expect("bundled travel-time table is valid"))
        .curve(depth_km)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(c: &Curve, dist: f64) -> (f64, f64) {
        let i = c.dist_km.iter().position(|d| *d == dist).unwrap();
        (c.p[i], c.s[i])
    }

    #[test]
    fn table_has_expected_shape() {
        let t = Table::parse(RAW).unwrap();
        assert_eq!((t.depths.len(), t.dists.len()), (106, 236));
        assert_eq!(*t.depths.last().unwrap(), 700.0);
        assert_eq!(*t.dists.last().unwrap(), 2000.0);
    }

    #[test]
    fn exact_mesh_values_match_the_source_file() {
        assert_eq!(at(&curve(0.0), 100.0), (17.683, 30.048));
        assert_eq!(at(&curve(100.0), 100.0), (19.497, 34.132));
        assert_eq!(at(&curve(100.0), 0.0), (13.927, 24.341));
    }

    #[test]
    fn depth_is_linearly_interpolated() {
        let (a, b, mid) = (curve(4.0), curve(6.0), curve(5.0));
        let ((pa, sa), (pb, sb), (pm, sm)) = (at(&a, 100.0), at(&b, 100.0), at(&mid, 100.0));
        assert!((pm - (pa + pb) / 2.0).abs() < 1e-9);
        assert!((sm - (sa + sb) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn depth_is_clamped_to_table_range() {
        assert_eq!(curve(-5.0), curve(0.0));
        assert_eq!(curve(900.0), curve(700.0));
    }

    #[test]
    fn times_increase_with_distance() {
        let c = curve(30.0);
        assert!(c.p.windows(2).all(|w| w[0] < w[1]));
        assert!(c.s.iter().zip(&c.p).all(|(s, p)| s > p));
    }
}
