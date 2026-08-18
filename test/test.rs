//! author: n685
//! date: 2026-08-18 18:41:20

use algo_lib::input::UnsafeScanner;
use std::io;
use std::io::{BufRead, BufWriter, Write};

fn tc<R: BufRead, W: Write>(scan: &mut UnsafeScanner<R>, out: &mut W, _st: &()) {
    let n: usize = scan.token();
    let m: usize = scan.token();
    let mut g = [false; 26];
    for _ in 0..n {
        let c = (scan.token::<String>().as_bytes()[0] - b'a') as usize;
        g[c] = true;
    }

    for _ in 0..m {
        let s = scan.token::<String>().into_bytes();
        if s.iter().any(|c| !g[(c - b'A') as usize]) {
            writeln!(out, "NO").ok();
            return;
        }
    }
    writeln!(out, "YES").ok();
}

fn solve<R: BufRead, W: Write>(scan: &mut UnsafeScanner<R>, out: &mut W) {
    let t: usize = scan.token();
    #[allow(unused_mut)]
    let mut st = ();
    for _ in 0..t {
        tc(scan, out, &st);
    }
}

fn main() {
    let mut scan = UnsafeScanner::new(io::stdin().lock());
    let mut out = BufWriter::new(io::stdout().lock());
    solve(&mut scan, &mut out);
}
