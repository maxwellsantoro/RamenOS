use std::path::PathBuf;

fn main() {
    let mut args = std::env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(command) = args.next() else {
        usage_and_exit(&program);
    };

    match command.to_string_lossy().as_ref() {
        "replay-trace" => {
            let Some(path) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            replay_trace(PathBuf::from(path));
        }
        "replay-blk-init-trace" => {
            let Some(path) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            replay_blk_init_trace(PathBuf::from(path));
        }
        "replay-sector-trace" => {
            let Some(path) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            replay_sector_trace(PathBuf::from(path));
        }
        "import-sector-jsonl" => {
            let Some(src) = args.next() else {
                usage_and_exit(&program);
            };
            let Some(dst) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            import_sector_jsonl(PathBuf::from(src), PathBuf::from(dst));
        }
        "assert-live-sector-trace" => {
            let Some(path) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            assert_live_sector_trace(PathBuf::from(path));
        }
        "import-jsonl" => {
            let Some(src) = args.next() else {
                usage_and_exit(&program);
            };
            let Some(dst) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            import_jsonl(PathBuf::from(src), PathBuf::from(dst));
        }
        "assert-live-trace" => {
            let Some(path) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            assert_live_trace(PathBuf::from(path));
        }
        "replay-packet-trace" => {
            let Some(path) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            replay_packet_trace(PathBuf::from(path));
        }
        "import-packet-jsonl" => {
            let Some(src) = args.next() else {
                usage_and_exit(&program);
            };
            let Some(dst) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            import_packet_jsonl(PathBuf::from(src), PathBuf::from(dst));
        }
        "assert-live-packet-trace" => {
            let Some(path) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            assert_live_packet_trace(PathBuf::from(path));
        }
        "assert-hardware-packet-trace" => {
            let Some(path) = args.next() else {
                usage_and_exit(&program);
            };
            if args.next().is_some() {
                usage_and_exit(&program);
            }
            assert_hardware_packet_trace(PathBuf::from(path));
        }
        _ => usage_and_exit(&program),
    }
}

fn replay_sector_trace(path: PathBuf) {
    match driver_foundry::replay_sector_trace_fixture(&path) {
        Ok(count) => {
            println!(
                "DRIVER_FOUNDRY_SECTOR_REPLAY: ok trace={} replay_events={}",
                path.display(),
                count
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_SECTOR_REPLAY: fail trace={} error={}",
                path.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn import_sector_jsonl(src: PathBuf, dst: PathBuf) {
    match driver_foundry::load_sector_jsonl(&src).and_then(|trace| {
        driver_foundry::write_sector_trace_json(&trace, &dst)?;
        Ok(trace.events.len())
    }) {
        Ok(count) => {
            println!(
                "DRIVER_FOUNDRY_SECTOR_IMPORT: ok src={} dst={} events={}",
                src.display(),
                dst.display(),
                count
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_SECTOR_IMPORT: fail src={} dst={} error={}",
                src.display(),
                dst.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn assert_live_sector_trace(path: PathBuf) {
    match driver_foundry::assert_live_sector_trace_file(&path) {
        Ok(()) => {
            println!(
                "DRIVER_FOUNDRY_LIVE_SECTOR_TRACE: ok trace={}",
                path.display()
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_LIVE_SECTOR_TRACE: fail trace={} error={}",
                path.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn replay_blk_init_trace(path: PathBuf) {
    match driver_foundry::load_trace(&path).and_then(|trace| {
        driver_foundry::virtio_blk_init::replay_init_trace(&trace)?;
        Ok(trace.events.len())
    }) {
        Ok(count) => {
            println!(
                "DRIVER_FOUNDRY_BLK_INIT_REPLAY: ok trace={} replay_events={}",
                path.display(),
                count
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_BLK_INIT_REPLAY: fail trace={} error={}",
                path.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn replay_trace(path: PathBuf) {
    match driver_foundry::replay_trace_fixture(&path) {
        Ok(count) => {
            println!(
                "DRIVER_FOUNDRY_REPLAY: ok trace={} replay_events={}",
                path.display(),
                count
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_REPLAY: fail trace={} error={}",
                path.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn import_jsonl(src: PathBuf, dst: PathBuf) {
    match driver_foundry::load_tracer_jsonl(&src).and_then(|trace| {
        driver_foundry::write_trace_json(&trace, &dst)?;
        Ok(trace.events.len())
    }) {
        Ok(count) => {
            println!(
                "DRIVER_FOUNDRY_IMPORT: ok src={} dst={} events={}",
                src.display(),
                dst.display(),
                count
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_IMPORT: fail src={} dst={} error={}",
                src.display(),
                dst.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn assert_live_trace(path: PathBuf) {
    match driver_foundry::assert_live_trace_file(&path) {
        Ok(()) => {
            println!("DRIVER_FOUNDRY_LIVE_TRACE: ok trace={}", path.display());
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_LIVE_TRACE: fail trace={} error={}",
                path.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn replay_packet_trace(path: PathBuf) {
    match driver_foundry::replay_packet_trace_fixture(&path) {
        Ok(count) => {
            println!(
                "DRIVER_FOUNDRY_PACKET_REPLAY: ok trace={} replay_events={}",
                path.display(),
                count
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_PACKET_REPLAY: fail trace={} error={}",
                path.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn import_packet_jsonl(src: PathBuf, dst: PathBuf) {
    match driver_foundry::load_packet_jsonl(&src).and_then(|trace| {
        driver_foundry::write_packet_trace_json(&trace, &dst)?;
        Ok(trace.events.len())
    }) {
        Ok(count) => {
            println!(
                "DRIVER_FOUNDRY_PACKET_IMPORT: ok src={} dst={} events={}",
                src.display(),
                dst.display(),
                count
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_PACKET_IMPORT: fail src={} dst={} error={}",
                src.display(),
                dst.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn assert_live_packet_trace(path: PathBuf) {
    match driver_foundry::assert_live_packet_trace_file(&path) {
        Ok(()) => {
            println!(
                "DRIVER_FOUNDRY_LIVE_PACKET_TRACE: ok trace={}",
                path.display()
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_LIVE_PACKET_TRACE: fail trace={} error={}",
                path.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn assert_hardware_packet_trace(path: PathBuf) {
    match driver_foundry::assert_hardware_packet_rx_file(&path) {
        Ok(()) => {
            println!(
                "DRIVER_FOUNDRY_HARDWARE_PACKET_TRACE: ok trace={}",
                path.display()
            );
        }
        Err(err) => {
            eprintln!(
                "DRIVER_FOUNDRY_HARDWARE_PACKET_TRACE: fail trace={} error={}",
                path.display(),
                err
            );
            std::process::exit(1);
        }
    }
}

fn usage_and_exit(program: &std::ffi::OsStr) -> ! {
    eprintln!(
        "usage: {} <replay-trace <driver_protocol_trace.json>|replay-blk-init-trace <driver_protocol_trace.json>|replay-sector-trace <block_sector_trace.json>|replay-packet-trace <net_packet_trace.json>|import-jsonl <tracer.jsonl> <out.json>|import-sector-jsonl <sector.jsonl> <out.json>|import-packet-jsonl <packet.jsonl> <out.json>|assert-live-trace <driver_protocol_trace.json>|assert-live-sector-trace <block_sector_trace.json>|assert-live-packet-trace <net_packet_trace.json>|assert-hardware-packet-trace <net_packet_trace.json>>",
        PathBuf::from(program).display()
    );
    std::process::exit(2);
}
