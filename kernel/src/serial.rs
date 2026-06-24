use core::fmt;
use core::fmt::Write;

use crate::arch;

pub fn init() {
    arch::serial::init();
}

struct Serial;

impl Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                arch::serial::write_byte(b'\r');
            }
            arch::serial::write_byte(byte);
        }
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    let mut serial = Serial;
    let _ = serial.write_fmt(args);
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::serial::_print(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! kprintln {
    () => {
        $crate::kprint!("\n")
    };
    ($fmt:expr) => {
        $crate::kprint!(concat!($fmt, "\n"))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::kprint!(concat!($fmt, "\n"), $($arg)*)
    };
}
