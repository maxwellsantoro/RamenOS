//! UEFI Graphics Output Protocol probe for S12.1 golden-machine bring-up.

use kernel::boot::{GOP_PROBE_FILL_FAILED, GOP_PROBE_OK, GopProbeInfo};
use uefi::prelude::*;
use uefi::proto::console::gop::{BltOp, BltPixel, GraphicsOutput, PixelFormat};

const GOP_FILL_WIDTH: usize = 64;
const GOP_FILL_HEIGHT: usize = 64;

pub fn probe_gop(st: &SystemTable<Boot>) -> GopProbeInfo {
    let bs = st.boot_services();
    let gop_handle = match bs.get_handle_for_protocol::<GraphicsOutput>() {
        Ok(handle) => handle,
        Err(_) => return GopProbeInfo::missing(),
    };

    let mut gop = match bs.open_protocol_exclusive::<GraphicsOutput>(gop_handle) {
        Ok(gop) => gop,
        Err(_) => return GopProbeInfo::missing(),
    };

    let mode_info = gop.current_mode_info();
    let (width, height) = mode_info.resolution();
    if width == 0 || height == 0 {
        return GopProbeInfo::missing();
    }

    let fill_w = GOP_FILL_WIDTH.min(width);
    let fill_h = GOP_FILL_HEIGHT.min(height);
    let fill_ok = gop
        .blt(BltOp::VideoFill {
            color: BltPixel::new(0x00, 0x40, 0x80),
            dest: (0, 0),
            dims: (fill_w, fill_h),
        })
        .is_ok();

    let status = if fill_ok {
        GOP_PROBE_OK
    } else {
        GOP_PROBE_FILL_FAILED
    };

    GopProbeInfo {
        status,
        width: width as u32,
        height: height as u32,
        pixel_format: pixel_format_raw(mode_info.pixel_format()),
        fill_ok,
    }
}

const fn pixel_format_raw(fmt: PixelFormat) -> u32 {
    match fmt {
        PixelFormat::Rgb => 0,
        PixelFormat::Bgr => 1,
        PixelFormat::Bitmask => 2,
        PixelFormat::BltOnly => 3,
    }
}
