pub mod server_status {
    tonic::include_proto!("server_status");
}

pub mod utils {
    const SYMBOLS_LEN: usize = 6;
    const SYMBOLS: [&str; SYMBOLS_LEN] = ["B", "K", "M", "G", "T", "P"];
    const fn init_units() -> [[u64; SYMBOLS_LEN]; 2] {
        let mut arr = [[0_u64; SYMBOLS_LEN]; 2];
        let mut idx = 0;
        while idx < SYMBOLS_LEN {
            arr[0][idx] = u64::pow(2, (idx * 10) as u32);
            arr[1][idx] = u64::pow(10, (idx * 3) as u32);
            idx += 1;
        }
        arr
    }

    const UNITS: [[u64; SYMBOLS_LEN]; 2] = init_units();
    pub fn bytes2human(value: u64, precision: usize, si: bool) -> String {
        let t: usize = if si { 1 } else { 0 };
        for idx in (0..SYMBOLS_LEN).rev() {
            let s = SYMBOLS[idx];
            if value >= UNITS[t][idx] {
                return format!("{:.*}{}", precision, value as f64 / UNITS[t][idx] as f64, s);
            }
        }
        format!("{:.*}B", precision, value as f64)
    }
}

#[allow(unused)]
#[cfg(test)]
mod tests {
    use crate::utils::bytes2human;

    #[test]
    fn test() {
        dbg!(bytes2human(536870912000, 2, false));
        dbg!(bytes2human(2048, 0, false));
        dbg!(bytes2human(0, 2, false));
        dbg!(bytes2human(1023, 0, false));
        dbg!(bytes2human(1024, 0, false));
        dbg!(bytes2human(u64::pow(1024, 2), 0, false));
        dbg!(bytes2human(u64::pow(1024, 3), 0, false));

        dbg!(bytes2human(100, 0, true));
        dbg!(bytes2human(1000, 0, true));
        dbg!(bytes2human(u64::pow(1000, 2), 0, true));
        dbg!(bytes2human(u64::pow(1000, 3), 0, true));
    }
}
