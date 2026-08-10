#![doc = "SCE-MAP: algorithm_cobs_encode:32 :: _forge_body"]
// SCE-MAP: algorithm_cobs_encode:32 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = false).

use sce_portable_bytes::{SceBytes, CapacityExceeded};

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn algorithm_cobs_encode(data: &[u8]) -> Result<SceBytes<32>, CapacityExceeded> {
    let n: u16 = (data).len() as u16;
    let mut out: SceBytes<32> = SceBytes::new();
    let mut p: u16 = 0;
    let mut done: bool = false;
    while done == false {
        let mut q: u16 = p;
        while q < n && q - p < 254 && data[(q) as usize] != 0 {
            q = q + 1;
        }
        let run: u16 = q - p;
        let code: u8 = (run + 1) as u8;
        out.push(code)?;
        let mut k: u16 = p;
        while k < q {
            out.push(data[(k) as usize])?;
            k = k + 1;
        }
        if q >= n {
            done = true;
        } else {
            if run < 254 {
                p = q + 1;
                if p >= n {
                    let last: u8 = 1;
                    out.push(last)?;
                    done = true;
                }
            } else {
                p = q;
            }
        }
    }
    return Ok(out);
}
