use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use owon_decode::{UartFrame, decode_uart_8n1};
use owon_device::{
    CaptureDebugInfo, CaptureOnceResult, DeviceSession, NormalizedCapture, UartBenchOptions,
};
use owon_protocol::{
    ChannelId, FLASH_SIZE, U32_RESPONSE_SIZE, fpga_state, machine_kind, pack_get_machine_command,
    pack_query_fpga_command, pack_read_flash_command, parse_flash_summary, parse_u32_response,
};
use owon_usb::{
    DEFAULT_USB_TIMEOUT, OpenSession, open_session, open_vds1022, scan_vds1022_devices,
};
use serde::Deserialize;
use serde_json::json;

const DEFAULT_UART_BAUD: f64 = 115_200.0;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Ok(ExitCode::FAILURE);
    };
    let trailing_args = args.collect::<Vec<_>>();

    match command.as_str() {
        "capture-once" => {
            if trailing_args.len() > 1 {
                print_usage();
                return Ok(ExitCode::FAILURE);
            }
            run_capture_once(trailing_args.first().map(String::as_str))
        }
        "uart-bench-capture" => run_uart_bench_capture(&trailing_args),
        "decode-uart" => run_decode_uart(&trailing_args),
        "bringup" if trailing_args.is_empty() => run_bringup(),
        "get-machine" if trailing_args.is_empty() => run_get_machine(),
        "load-fpga" if trailing_args.is_empty() => run_load_fpga(),
        "open" if trailing_args.is_empty() => run_open(),
        "permissions" if trailing_args.is_empty() => run_permissions(),
        "query-fpga" if trailing_args.is_empty() => run_query_fpga(),
        "read-flash" if trailing_args.is_empty() => run_read_flash(),
        "status" if trailing_args.is_empty() => run_status(),
        "-h" | "--help" | "help" => {
            print_usage();
            Ok(ExitCode::SUCCESS)
        }
        _ => {
            print_usage();
            Ok(ExitCode::FAILURE)
        }
    }
}

fn run_status() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let devices = scan_vds1022_devices()?;
    if devices.is_empty() {
        println!("No OWON VDS1022 devices found.");
        return Ok(ExitCode::FAILURE);
    }

    println!("Found {} OWON VDS1022 device(s).", devices.len());
    for (index, device) in devices.iter().enumerate() {
        if index > 0 {
            println!();
        }

        println!("Device {}", index + 1);
        println!("  Source: {}", device.backend);
        println!(
            "  Bus/device: {:03}/{:03}",
            device.bus_number, device.address
        );
        println!("  Bus path: {}", device.bus_device_path());
        println!(
            "  VID:PID: {:04x}:{:04x}",
            device.vendor_id, device.product_id
        );
        println!(
            "  Device descriptor: class={:02x} subclass={:02x} protocol={:02x}",
            device.device_class, device.device_sub_class, device.device_protocol
        );
        println!("  Max packet size (ep0): {}", device.max_packet_size_0);
        println!("  Configurations: {}", device.num_configurations);
        println!(
            "  USB version: {}",
            device.usb_version.as_deref().unwrap_or("?")
        );
        println!(
            "  Device release: {}",
            device.device_release.as_deref().unwrap_or("?")
        );
        println!(
            "  Speed (Mb/s): {}",
            device.speed_mbps.as_deref().unwrap_or("?")
        );
        println!(
            "  Manufacturer: {}",
            device.manufacturer.as_deref().unwrap_or("?")
        );
        println!("  Product: {}", device.product.as_deref().unwrap_or("?"));
        println!("  Serial: {}", device.serial.as_deref().unwrap_or("?"));
        println!(
            "  Sysfs device: {}",
            device.sysfs_name.as_deref().unwrap_or("?")
        );
        println!(
            "  Sysfs interface: {}",
            device.interface_sysfs_name.as_deref().unwrap_or("?")
        );
        println!(
            "  Active driver: {}",
            device.active_driver().unwrap_or("none")
        );
        println!(
            "  Expected OWON layout: {}",
            if device.matches_expected_layout() {
                "yes"
            } else {
                "no"
            }
        );

        for interface in &device.interfaces {
            println!(
                "  Interface {} alt {} class={:02x}/{:02x}/{:02x}",
                interface.number,
                interface.alternate_setting,
                interface.class_code,
                interface.sub_class_code,
                interface.protocol_code
            );
            println!(
                "    Driver: {}",
                interface.driver.as_deref().unwrap_or("none")
            );
            println!(
                "    Modalias: {}",
                interface.modalias.as_deref().unwrap_or("?")
            );
            for endpoint in &interface.endpoints {
                println!(
                    "    Endpoint 0x{:02x}: {} {} max_packet={} interval={}",
                    endpoint.address,
                    endpoint.direction,
                    endpoint.transfer_type,
                    endpoint.max_packet_size,
                    endpoint.interval
                );
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn run_open() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let report = open_vds1022()?;

    println!("OWON open test succeeded.");
    println!(
        "  Bus/device: {:03}/{:03}",
        report.snapshot.bus_number, report.snapshot.address
    );
    println!("  Device node: {}", report.device_node_path.display());
    println!("  Source: {}", report.snapshot.backend);
    println!(
        "  Active driver before open: {}",
        report.snapshot.active_driver().unwrap_or("none")
    );
    println!(
        "  Detach supported: {}",
        if report.detach_supported { "yes" } else { "no" }
    );
    println!(
        "  Kernel driver active at open: {}",
        if report.kernel_driver_was_active {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "  Detached kernel driver: {}",
        if report.detached_kernel_driver {
            "yes"
        } else {
            "no"
        }
    );
    println!("  Claimed interface: {}", report.claimed_interface);
    println!(
        "  Released interface: {}",
        if report.released_interface {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "  Reattached kernel driver: {}",
        if report.reattached_kernel_driver {
            "yes"
        } else {
            "no"
        }
    );

    Ok(ExitCode::SUCCESS)
}

fn run_permissions() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let devices = scan_vds1022_devices()?;
    if devices.is_empty() {
        println!("No OWON VDS1022 devices found.");
        println!("Install the udev rule first if needed, then plug the device back in.");
        println!("Suggested command: sudo bash scripts/install-udev-rule.sh");
        return Ok(ExitCode::FAILURE);
    }

    let device = &devices[0];
    let device_node = device.device_node_path();

    println!("OWON permission check");
    println!(
        "  Bus/device: {:03}/{:03}",
        device.bus_number, device.address
    );
    println!("  Device node: {}", device_node.display());
    println!(
        "  Active driver: {}",
        device.active_driver().unwrap_or("none")
    );
    println!("  Suggested install command: sudo bash scripts/install-udev-rule.sh");

    if let Some(rule) = first_existing_rule() {
        println!("  Installed rule: {}", rule.display());
    } else {
        println!("  Installed rule: none found in common locations");
    }

    if device_node.exists() {
        let metadata = fs::metadata(&device_node)?;
        let mode = metadata.mode() & 0o777;
        println!("  Device node mode: {:03o}", mode);
        println!(
            "  Device node uid:gid = {}:{}",
            metadata.uid(),
            metadata.gid()
        );
    } else {
        println!("  Device node visibility: not visible in this environment");
    }

    println!("  Next steps:");
    println!("    1. sudo bash scripts/install-udev-rule.sh");
    println!("    2. unplug and replug the scope");
    println!("    3. cargo run -p owon-probe -- open");

    Ok(ExitCode::SUCCESS)
}

fn run_get_machine() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let session = open_session()?;
    let parsed = execute_u32_command(&session, &pack_get_machine_command())?;
    let machine = machine_kind(parsed.value);

    println!("GET_MACHINE succeeded.");
    println!(
        "  Status byte: 0x{:02x} ({})",
        parsed.status,
        printable_byte(parsed.status)
    );
    println!("  Raw value: {}", parsed.value);
    println!("  Machine: {}", machine);

    session.close()?;
    Ok(ExitCode::SUCCESS)
}

fn run_query_fpga() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let session = open_session()?;
    let parsed = execute_u32_command(&session, &pack_query_fpga_command())?;
    let state = fpga_state(parsed.value);

    println!("QUERY_FPGA succeeded.");
    println!(
        "  Status byte: 0x{:02x} ({})",
        parsed.status,
        printable_byte(parsed.status)
    );
    println!("  Raw value: {}", parsed.value);
    println!("  FPGA state: {}", state);

    session.close()?;
    Ok(ExitCode::SUCCESS)
}

fn run_read_flash() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let session = open_session()?;
    let response = read_flash_bytes(&session)?;
    let summary = parse_flash_summary(&response)?;

    println!("READ_FLASH succeeded.");
    println!("  Bytes: {}", response.len());
    println!("  Header: 0x{:04x}", summary.header);
    println!("  Flash version: {}", summary.version);
    println!("  OEM: {}", summary.oem);
    println!("  Device version: {}", summary.device_version);
    println!("  Serial: {}", summary.serial);
    println!("  Non-zero locale bytes: {}", summary.nonzero_locale_bytes);
    println!("  Phase fine: {}", summary.phasefine);
    println!("  Inferred FPGA version: {}", summary.inferred_vfpga);

    session.close()?;
    Ok(ExitCode::SUCCESS)
}

fn run_load_fpga() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let session = DeviceSession::open()?;
    let info = session.info().clone();

    if !info.firmware_loaded_this_session {
        println!("LOAD_FPGA skipped.");
        println!("  FPGA state: already loaded");
        session.close()?;
        return Ok(ExitCode::SUCCESS);
    }

    println!("LOAD_FPGA succeeded.");
    println!("  Device version: {}", info.flash.device_version);
    println!("  Serial: {}", info.flash.serial);
    println!("  FPGA state before: {}", info.fpga_state_before);
    println!("  FPGA state after: {}", info.fpga_state_after);
    println!(
        "  Loaded in this session: {}",
        if info.firmware_loaded_this_session {
            "yes"
        } else {
            "no"
        }
    );
    if let Some(path) = &info.selected_firmware {
        println!("  Selected image: {}", path.display());
    }

    session.close()?;
    Ok(ExitCode::SUCCESS)
}

fn run_bringup() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let session = DeviceSession::open()?;
    let info = session.info().clone();

    println!("BRINGUP succeeded.");
    println!("  Device version: {}", info.flash.device_version);
    println!("  Serial: {}", info.flash.serial);
    println!("  Inferred FPGA version: {}", info.flash.inferred_vfpga);
    println!("  FPGA state before: {}", info.fpga_state_before);
    println!("  FPGA state after: {}", info.fpga_state_after);
    if let Some(path) = &info.selected_firmware {
        println!("  Loaded image: {}", path.display());
    }
    println!(
        "  Firmware loaded in this session: {}",
        if info.firmware_loaded_this_session {
            "yes"
        } else {
            "no"
        }
    );
    println!("  Machine: {}", info.machine);

    session.close()?;
    Ok(ExitCode::SUCCESS)
}

fn run_capture_once(output_prefix: Option<&str>) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let prefix = output_prefix.unwrap_or("owon-capture");
    let outputs = CaptureOutputPaths::from_prefix(prefix);
    if let Some(parent) = outputs.meta.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut session = DeviceSession::open()?;
    let info = session.info().clone();
    let capture = session.capture_once()?;
    let ch1 = capture
        .channel(ChannelId::Ch1)
        .ok_or_else(|| io::Error::other("capture missing CH1"))?;
    let ch2 = capture
        .channel(ChannelId::Ch2)
        .ok_or_else(|| io::Error::other("capture missing CH2"))?;

    fs::write(&outputs.ch1, &ch1.samples)?;
    fs::write(&outputs.ch2, &ch2.samples)?;
    fs::write(
        &outputs.meta,
        render_capture_metadata_json(&capture, &outputs),
    )?;

    println!("CAPTURE_ONCE succeeded.");
    println!("  Device version: {}", info.flash.device_version);
    println!("  Serial: {}", info.flash.serial);
    println!("  Sample rate: {} Hz", capture.sample_rate_hz);
    println!("  Sample period: {} ns", capture.sample_period_ns);
    println!("  Channels: {}", capture.channels.len());
    for channel in [&ChannelId::Ch1, &ChannelId::Ch2] {
        let capture = capture
            .channel(*channel)
            .ok_or_else(|| io::Error::other(format!("capture missing {channel}")))?;
        println!("  {} sample count: {}", channel, capture.samples.len());
        println!(
            "  {} trigger index: {}",
            channel,
            match capture.trigger_index {
                Some(value) => value.to_string(),
                None => "none".to_owned(),
            }
        );
        println!(
            "  {} frequency: {}",
            channel,
            match capture.frequency_hz {
                Some(value) => value.to_string(),
                None => "unknown".to_owned(),
            }
        );
    }
    println!("  Metadata: {}", outputs.meta.display());
    println!("  CH1 raw dump: {}", outputs.ch1.display());
    println!("  CH2 raw dump: {}", outputs.ch2.display());

    session.close()?;
    Ok(ExitCode::SUCCESS)
}

fn run_uart_bench_capture(
    trailing_args: &[String],
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut prefix = "uart-bench".to_owned();
    let mut options = UartBenchOptions::for_channel(ChannelId::Ch1);
    let mut positional_count = 0usize;
    let mut index = 0usize;

    while index < trailing_args.len() {
        let arg = &trailing_args[index];
        if !arg.starts_with("--") {
            if let Some(channel) = parse_channel_name(arg) {
                options.channel = channel.channel_id();
            } else if positional_count == 0 {
                prefix = arg.clone();
                positional_count += 1;
            } else {
                return Err(io::Error::other(format!("unexpected argument: {arg}")).into());
            }
            index += 1;
            continue;
        }

        match arg.as_str() {
            "--volt-range-index" => {
                options.volt_range_index = parse_usize_arg(&trailing_args[index + 1..], arg)?;
                index += 2;
            }
            "--timebase-prescaler" => {
                options.timebase_prescaler = parse_u32_arg(&trailing_args[index + 1..], arg)?;
                index += 2;
            }
            "--pre-trigger" => {
                options.pre_trigger_samples = parse_u16_arg(&trailing_args[index + 1..], arg)?;
                index += 2;
            }
            "--post-trigger" => {
                options.post_trigger_samples = parse_u32_arg(&trailing_args[index + 1..], arg)?;
                index += 2;
            }
            "--holdoff" => {
                options.trigger_holdoff = parse_u16_arg(&trailing_args[index + 1..], arg)?;
                index += 2;
            }
            "--edge-level" => {
                options.edge_level = parse_u16_arg(&trailing_args[index + 1..], arg)?;
                index += 2;
            }
            "--freqref" => {
                options.freqref = parse_u8_arg(&trailing_args[index + 1..], arg)?;
                index += 2;
            }
            "--rollmode" => {
                options.rollmode = true;
                index += 1;
            }
            "--peakmode" => {
                options.peakmode = true;
                index += 1;
            }
            _ => return Err(io::Error::other(format!("unsupported option: {arg}")).into()),
        }
    }

    let outputs = CaptureOutputPaths::from_prefix(&prefix);
    create_output_dirs(&outputs)?;

    let mut session = DeviceSession::open()?;
    let info = session.info().clone();
    let result = session.capture_uart_bench_with_options(options)?;
    write_capture_result(&outputs, &result)?;

    println!("UART_BENCH_CAPTURE succeeded.");
    println!("  Device version: {}", info.flash.device_version);
    println!("  Serial: {}", info.flash.serial);
    println!("  Requested channel: {}", options.channel.short_name());
    println!("  Sample rate: {} Hz", result.capture.sample_rate_hz);
    println!("  Sample period: {} ns", result.capture.sample_period_ns);
    println!("  Volt range index: {}", result.debug.volt_range_index);
    println!("  Timebase prescaler: {}", result.debug.timebase_prescaler);
    println!(
        "  Pre-trigger samples: {}",
        result.debug.pre_trigger_samples
    );
    println!(
        "  Post-trigger samples: {}",
        result.debug.post_trigger_samples
    );
    println!("  Trigger holdoff: 0x{:04x}", result.debug.trigger_holdoff);
    println!("  Edge level: 0x{:04x}", result.debug.edge_level);
    println!("  Freqref: 0x{:02x}", result.debug.freqref);
    println!(
        "  Roll mode: {}",
        if result.debug.rollmode { "yes" } else { "no" }
    );
    println!(
        "  Peak mode: {}",
        if result.debug.peakmode { "yes" } else { "no" }
    );
    println!("  Metadata: {}", outputs.meta.display());
    println!("  Debug: {}", outputs.debug.display());
    for channel in &result.capture.channels {
        println!(
            "  {} raw dump: {}",
            channel.channel,
            outputs.path_for_channel(channel.channel).display()
        );
    }
    for channel in &result.debug.channels {
        println!(
            "  {} stats: offset={} u8=[{}, {}] i8=[{}, {}] cursor={} time_sum={} period_num={}",
            channel.channel,
            channel.sample_offset,
            channel.sample_min_u8,
            channel.sample_max_u8,
            channel.sample_min_i8,
            channel.sample_max_i8,
            channel.cursor_from_right,
            channel.time_sum,
            channel.period_num,
        );
    }

    session.close()?;
    Ok(ExitCode::SUCCESS)
}

fn run_decode_uart(trailing_args: &[String]) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let (metadata_path, requested_channel, baud, options) = parse_decode_uart_args(trailing_args)?;
    let metadata = load_capture_metadata(&metadata_path)?;
    let channel = metadata
        .channel(requested_channel.short_name())
        .ok_or_else(|| {
            io::Error::other(format!(
                "metadata does not contain {}",
                requested_channel.short_name()
            ))
        })?;
    let samples_path = resolve_samples_path(&metadata_path, &channel.samples_file);
    let raw_samples = fs::read(&samples_path)?;
    if raw_samples.len() != channel.sample_count {
        return Err(io::Error::other(format!(
            "sample count mismatch for {}: metadata says {}, file has {}",
            requested_channel.short_name(),
            channel.sample_count,
            raw_samples.len()
        ))
        .into());
    }

    let report = decode_uart_capture(&raw_samples, metadata.sample_rate_hz, baud, options)?;

    println!("DECODE_UART succeeded.");
    println!("  Metadata: {}", metadata_path.display());
    println!("  Channel: {}", requested_channel.short_name());
    println!("  Raw samples: {}", raw_samples.len());
    println!("  Sample rate: {} Hz", metadata.sample_rate_hz);
    println!("  Baud: {}", baud);
    println!("  Threshold (i8): {}", report.threshold);
    println!(
        "  Raw range (u8): {}..{}",
        report.min_sample_u8, report.max_sample_u8
    );
    println!(
        "  Signed range (i8): {}..{}",
        report.min_sample_i8, report.max_sample_i8
    );
    println!("  Invert: {}", if options.invert { "yes" } else { "no" });
    println!("  Frames: {}", report.frames.len());
    println!("  Framing errors: {}", report.framing_errors());
    println!("  Bytes: {}", render_uart_bytes(&report.frames));
    println!("  ASCII: {}", render_uart_ascii(&report.frames));
    if !report.frames.is_empty() && report.framing_errors() == report.frames.len() {
        println!(
            "  Warning: every decoded frame failed the stop-bit check; this usually means the waveform, threshold, or polarity still needs bench tuning."
        );
    }

    for frame in &report.frames {
        println!(
            "  Frame @{}: 0x{:02x} {} stop={}",
            frame.start_sample,
            frame.data,
            frame.ascii.unwrap_or('.'),
            if frame.stop_ok { "ok" } else { "bad" }
        );
    }

    Ok(if report.frames.is_empty() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn first_existing_rule() -> Option<&'static Path> {
    common_rule_paths().into_iter().find(|path| path.exists())
}

fn common_rule_paths() -> [&'static Path; 4] {
    [
        Path::new("/etc/udev/rules.d/70-owon-vds1022.rules"),
        Path::new("/etc/udev/rules.d/70-owon-vds-tiny.rules"),
        Path::new("/usr/lib/udev/rules.d/70-owon-vds1022.rules"),
        Path::new("/usr/lib/udev/rules.d/70-owon-vds-tiny.rules"),
    ]
}

fn print_usage() {
    eprintln!(
        "Usage: owon-probe <status|permissions|open|get-machine|query-fpga|read-flash|load-fpga|bringup|capture-once [output-prefix]|uart-bench-capture [output-prefix] [ch1|ch2] [--volt-range-index <n>] [--timebase-prescaler <n>] [--pre-trigger <n>] [--post-trigger <n>] [--holdoff <u16>] [--edge-level <u16>] [--freqref <u8>] [--rollmode] [--peakmode]|decode-uart <meta.json> <ch1|ch2> [baud] [--threshold <signed-i8>] [--invert]>"
    );
}

fn parse_integer_arg(args: &[String], option: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let value = args
        .first()
        .ok_or_else(|| io::Error::other(format!("missing value for {option}")))?;
    let (radix, digits) = if let Some(rest) = value.strip_prefix("0x") {
        (16, rest)
    } else if let Some(rest) = value.strip_prefix("0X") {
        (16, rest)
    } else {
        (10, value.as_str())
    };

    u64::from_str_radix(digits, radix)
        .map_err(|error| io::Error::other(format!("invalid value for {option}: {error}")).into())
}

fn parse_usize_arg(args: &[String], option: &str) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(usize::try_from(parse_integer_arg(args, option)?)
        .map_err(|_| io::Error::other(format!("value out of range for {option}")))?)
}

fn parse_u32_arg(args: &[String], option: &str) -> Result<u32, Box<dyn std::error::Error>> {
    Ok(u32::try_from(parse_integer_arg(args, option)?)
        .map_err(|_| io::Error::other(format!("value out of range for {option}")))?)
}

fn parse_u16_arg(args: &[String], option: &str) -> Result<u16, Box<dyn std::error::Error>> {
    Ok(u16::try_from(parse_integer_arg(args, option)?)
        .map_err(|_| io::Error::other(format!("value out of range for {option}")))?)
}

fn parse_u8_arg(args: &[String], option: &str) -> Result<u8, Box<dyn std::error::Error>> {
    Ok(u8::try_from(parse_integer_arg(args, option)?)
        .map_err(|_| io::Error::other(format!("value out of range for {option}")))?)
}

fn printable_byte(byte: u8) -> String {
    if byte.is_ascii_graphic() || byte == b' ' {
        (byte as char).to_string()
    } else {
        "?".to_owned()
    }
}

fn execute_u32_command(
    session: &OpenSession,
    request: &[u8],
) -> Result<owon_protocol::U32Response, Box<dyn std::error::Error>> {
    let mut response = [0u8; U32_RESPONSE_SIZE];
    session.write_bulk_exact(request, DEFAULT_USB_TIMEOUT)?;
    session.read_bulk_exact(&mut response, DEFAULT_USB_TIMEOUT)?;
    Ok(parse_u32_response(&response)?)
}

fn read_flash_bytes(session: &OpenSession) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let request = pack_read_flash_command();
    let mut response = vec![0u8; FLASH_SIZE];
    session.write_bulk_exact(&request, DEFAULT_USB_TIMEOUT)?;
    session.read_bulk_exact(&mut response, DEFAULT_USB_TIMEOUT)?;
    Ok(response)
}

#[derive(Debug, Clone)]
struct CaptureOutputPaths {
    meta: PathBuf,
    debug: PathBuf,
    ch1: PathBuf,
    ch2: PathBuf,
}

impl CaptureOutputPaths {
    fn from_prefix(prefix: &str) -> Self {
        let prefix = PathBuf::from(prefix);
        Self {
            meta: append_suffix(&prefix, "-meta.json"),
            debug: append_suffix(&prefix, "-debug.json"),
            ch1: append_suffix(&prefix, "-ch1.bin"),
            ch2: append_suffix(&prefix, "-ch2.bin"),
        }
    }

    fn path_for_channel(&self, channel: ChannelId) -> &Path {
        match channel {
            ChannelId::Ch1 => &self.ch1,
            ChannelId::Ch2 => &self.ch2,
        }
    }
}

fn append_suffix(prefix: &Path, suffix: &str) -> PathBuf {
    let mut value = OsString::from(prefix.as_os_str());
    value.push(suffix);
    PathBuf::from(value)
}

fn create_output_dirs(outputs: &CaptureOutputPaths) -> io::Result<()> {
    for path in [&outputs.meta, &outputs.debug, &outputs.ch1, &outputs.ch2] {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
    }
    Ok(())
}

fn write_capture_result(
    outputs: &CaptureOutputPaths,
    result: &CaptureOnceResult,
) -> Result<(), Box<dyn std::error::Error>> {
    for channel in &result.capture.channels {
        fs::write(outputs.path_for_channel(channel.channel), &channel.samples)?;
    }
    fs::write(
        &outputs.meta,
        render_capture_metadata_json(&result.capture, outputs),
    )?;
    fs::write(&outputs.debug, render_capture_debug_json(&result.debug))?;
    Ok(())
}

fn render_capture_metadata_json(
    capture: &NormalizedCapture,
    outputs: &CaptureOutputPaths,
) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!(
        "  \"sample_rate_hz\": {},\n  \"sample_period_ns\": {},\n  \"channels\": [\n",
        capture.sample_rate_hz, capture.sample_period_ns
    ));

    for (index, channel) in capture.channels.iter().enumerate() {
        if index > 0 {
            json.push_str(",\n");
        }
        json.push_str("    {\n");
        json.push_str(&format!(
            "      \"channel\": \"{}\",\n      \"samples_file\": \"{}\",\n      \"sample_count\": {},\n      \"trigger_index\": {},\n      \"frequency_hz\": {}\n",
            channel.channel.short_name(),
            json_escape(&outputs.path_for_channel(channel.channel).to_string_lossy()),
            channel.samples.len(),
            format_optional_usize(channel.trigger_index),
            format_optional_f64(channel.frequency_hz),
        ));
        json.push_str("    }");
    }

    json.push_str("\n  ]\n}\n");
    json
}

fn format_optional_usize(value: Option<usize>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    }
}

fn format_optional_f64(value: Option<f64>) -> String {
    match value {
        Some(value) if value.is_finite() => value.to_string(),
        _ => "null".to_owned(),
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => escaped.push(other),
        }
    }
    escaped
}

fn render_capture_debug_json(debug: &CaptureDebugInfo) -> String {
    let value = json!({
        "requested_channels": debug.requested_channels.iter().map(|channel| channel.short_name()).collect::<Vec<_>>(),
        "volt_range_index": debug.volt_range_index,
        "timebase_prescaler": debug.timebase_prescaler,
        "rollmode": debug.rollmode,
        "peakmode": debug.peakmode,
        "pre_trigger_samples": debug.pre_trigger_samples,
        "post_trigger_samples": debug.post_trigger_samples,
        "trigger_holdoff": debug.trigger_holdoff,
        "edge_level": debug.edge_level,
        "freqref": debug.freqref,
        "phasefine": debug.phasefine,
        "calibrations": debug.calibrations.iter().map(|calibration| json!({
            "channel": calibration.channel.short_name(),
            "zero_offset": calibration.zero_offset,
            "voltage_gain": calibration.voltage_gain,
        })).collect::<Vec<_>>(),
        "channels": debug.channels.iter().map(|channel| json!({
            "channel": channel.channel.short_name(),
            "cursor_from_right": channel.cursor_from_right,
            "time_sum": channel.time_sum,
            "period_num": channel.period_num,
            "sample_offset": channel.sample_offset,
            "sample_count": channel.sample_count,
            "sample_min_u8": channel.sample_min_u8,
            "sample_max_u8": channel.sample_max_u8,
            "sample_min_i8": channel.sample_min_i8,
            "sample_max_i8": channel.sample_max_i8,
        })).collect::<Vec<_>>(),
    });

    serde_json::to_string_pretty(&value).expect("debug json serialization should succeed")
}

#[derive(Debug, Deserialize)]
struct CaptureMetadata {
    sample_rate_hz: f64,
    channels: Vec<CaptureMetadataChannel>,
}

impl CaptureMetadata {
    fn channel(&self, name: &str) -> Option<&CaptureMetadataChannel> {
        self.channels.iter().find(|channel| channel.channel == name)
    }
}

#[derive(Debug, Deserialize)]
struct CaptureMetadataChannel {
    channel: String,
    samples_file: String,
    sample_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeChannel {
    Ch1,
    Ch2,
}

impl ProbeChannel {
    fn short_name(self) -> &'static str {
        match self {
            Self::Ch1 => "ch1",
            Self::Ch2 => "ch2",
        }
    }

    fn channel_id(self) -> ChannelId {
        match self {
            Self::Ch1 => ChannelId::Ch1,
            Self::Ch2 => ChannelId::Ch2,
        }
    }
}

#[derive(Debug)]
struct UartDecodeReport {
    threshold: i8,
    min_sample_u8: u8,
    max_sample_u8: u8,
    min_sample_i8: i8,
    max_sample_i8: i8,
    frames: Vec<UartFrame>,
}

impl UartDecodeReport {
    fn framing_errors(&self) -> usize {
        self.frames.iter().filter(|frame| !frame.stop_ok).count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UartDecodeOptions {
    threshold_override: Option<i8>,
    invert: bool,
}

fn load_capture_metadata(path: &Path) -> Result<CaptureMetadata, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn resolve_samples_path(metadata_path: &Path, samples_file: &str) -> PathBuf {
    let samples_path = PathBuf::from(samples_file);
    if samples_path.is_absolute() {
        samples_path
    } else {
        metadata_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(samples_path)
    }
}

fn parse_channel_name(value: &str) -> Option<ProbeChannel> {
    match value.to_ascii_lowercase().as_str() {
        "ch1" => Some(ProbeChannel::Ch1),
        "ch2" => Some(ProbeChannel::Ch2),
        _ => None,
    }
}

fn decode_uart_capture(
    raw_samples: &[u8],
    sample_rate_hz: f64,
    baud: f64,
    options: UartDecodeOptions,
) -> Result<UartDecodeReport, Box<dyn std::error::Error>> {
    let (min_sample_u8, max_sample_u8) = raw_sample_range(raw_samples)
        .ok_or_else(|| io::Error::other("cannot decode UART from an empty sample buffer"))?;
    let (min_sample_i8, max_sample_i8) = signed_sample_range(raw_samples)
        .ok_or_else(|| io::Error::other("cannot decode UART from an empty sample buffer"))?;
    let threshold = options
        .threshold_override
        .unwrap_or_else(|| midpoint_threshold_i8(min_sample_i8, max_sample_i8));
    let digital_samples = threshold_signed_samples(raw_samples, threshold);
    let frames = decode_uart_8n1(&digital_samples, sample_rate_hz, baud, options.invert)?;

    Ok(UartDecodeReport {
        threshold,
        min_sample_u8,
        max_sample_u8,
        min_sample_i8,
        max_sample_i8,
        frames,
    })
}

fn raw_sample_range(samples: &[u8]) -> Option<(u8, u8)> {
    let min = samples.iter().copied().min()?;
    let max = samples.iter().copied().max()?;
    Some((min, max))
}

fn signed_sample_range(samples: &[u8]) -> Option<(i8, i8)> {
    let min = samples.iter().copied().map(|sample| sample as i8).min()?;
    let max = samples.iter().copied().map(|sample| sample as i8).max()?;
    Some((min, max))
}

fn midpoint_threshold_i8(min_sample: i8, max_sample: i8) -> i8 {
    ((i16::from(min_sample) + i16::from(max_sample)) / 2) as i8
}

fn threshold_signed_samples(raw_samples: &[u8], threshold: i8) -> Vec<u8> {
    raw_samples
        .iter()
        .map(|&sample| u8::from((sample as i8) > threshold))
        .collect()
}

fn parse_decode_uart_args(
    trailing_args: &[String],
) -> Result<(PathBuf, ProbeChannel, f64, UartDecodeOptions), Box<dyn std::error::Error>> {
    if trailing_args.len() < 2 {
        print_usage();
        return Err(io::Error::other("decode-uart requires at least <meta.json> <ch1|ch2>").into());
    }

    let metadata_path = PathBuf::from(&trailing_args[0]);
    let requested_channel = parse_channel_name(&trailing_args[1])
        .ok_or_else(|| io::Error::other(format!("unsupported channel: {}", trailing_args[1])))?;

    let mut baud = DEFAULT_UART_BAUD;
    let mut threshold_override = None;
    let mut invert = false;
    let mut index = 2usize;

    while index < trailing_args.len() {
        match trailing_args[index].as_str() {
            "--invert" => {
                invert = true;
                index += 1;
            }
            "--threshold" => {
                let value = trailing_args
                    .get(index + 1)
                    .ok_or_else(|| io::Error::other("--threshold requires a signed i8 value"))?;
                threshold_override = Some(parse_signed_i8(value)?);
                index += 2;
            }
            value => {
                if (baud - DEFAULT_UART_BAUD).abs() > f64::EPSILON {
                    return Err(io::Error::other(format!(
                        "unexpected decode-uart argument: {value}"
                    ))
                    .into());
                }
                baud = value.parse::<f64>()?;
                index += 1;
            }
        }
    }

    Ok((
        metadata_path,
        requested_channel,
        baud,
        UartDecodeOptions {
            threshold_override,
            invert,
        },
    ))
}

fn parse_signed_i8(value: &str) -> Result<i8, Box<dyn std::error::Error>> {
    let parsed = value.parse::<i16>()?;
    let converted = i8::try_from(parsed)
        .map_err(|_| io::Error::other(format!("threshold out of i8 range: {parsed}")))?;
    Ok(converted)
}

fn render_uart_bytes(frames: &[UartFrame]) -> String {
    if frames.is_empty() {
        return "(none)".to_owned();
    }

    frames
        .iter()
        .map(|frame| format!("{:02x}", frame.data))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_uart_ascii(frames: &[UartFrame]) -> String {
    if frames.is_empty() {
        return "(none)".to_owned();
    }

    frames
        .iter()
        .map(|frame| frame.ascii.unwrap_or('.'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use owon_device::ChannelCapture;

    #[test]
    fn render_capture_metadata_json_lists_both_channel_dumps() {
        let capture = NormalizedCapture {
            sample_rate_hz: 2_500_000.0,
            sample_period_ns: 400.0,
            channels: vec![
                ChannelCapture {
                    channel: ChannelId::Ch1,
                    samples: vec![1, 2, 3],
                    trigger_index: Some(0),
                    frequency_hz: Some(115_200.0),
                },
                ChannelCapture {
                    channel: ChannelId::Ch2,
                    samples: vec![4, 5, 6],
                    trigger_index: Some(0),
                    frequency_hz: None,
                },
            ],
        };
        let outputs = CaptureOutputPaths::from_prefix("uart-test");

        let json = render_capture_metadata_json(&capture, &outputs);
        assert!(json.contains("\"sample_rate_hz\": 2500000"));
        assert!(json.contains("\"samples_file\": \"uart-test-ch1.bin\""));
        assert!(json.contains("\"samples_file\": \"uart-test-ch2.bin\""));
        assert!(json.contains("\"trigger_index\": 0"));
        assert!(json.contains("\"frequency_hz\": 115200"));
        assert!(json.contains("\"frequency_hz\": null"));
    }

    #[test]
    fn threshold_bridge_decodes_known_uart_fixture() {
        let samples = fs::read(testdata_root().join("samples.bin")).expect("fixture samples");
        let report = decode_uart_capture(
            &samples,
            10_000_000.0,
            115_200.0,
            UartDecodeOptions {
                threshold_override: None,
                invert: false,
            },
        )
        .expect("UART decode over fixture should succeed");

        assert_eq!(report.threshold, 0);
        assert_eq!(report.min_sample_u8, 0);
        assert_eq!(report.max_sample_u8, 1);
        assert_eq!(report.min_sample_i8, 0);
        assert_eq!(report.max_sample_i8, 1);
        assert_eq!(report.framing_errors(), 0);
        assert_eq!(
            render_uart_bytes(&report.frames),
            "39 41 41 20 4c 61 79 65 72 20 6d 75 73 74 20 62 65 20 73 65 6c 65 63 74 65 64 2e 00"
        );
        assert_eq!(
            render_uart_ascii(&report.frames),
            "9AA Layer must be selected.."
        );
    }

    #[test]
    fn signed_sample_range_respects_wrapped_negative_values() {
        let (min_sample, max_sample) = signed_sample_range(&[0, 1, 2, 255, 254]).expect("range");
        assert_eq!(min_sample, -2);
        assert_eq!(max_sample, 2);
        assert_eq!(midpoint_threshold_i8(min_sample, max_sample), 0);
    }

    #[test]
    fn render_capture_metadata_json_handles_single_channel_capture() {
        let capture = NormalizedCapture {
            sample_rate_hz: 2_500_000.0,
            sample_period_ns: 400.0,
            channels: vec![ChannelCapture {
                channel: ChannelId::Ch1,
                samples: vec![1, 2, 3],
                trigger_index: Some(0),
                frequency_hz: Some(115_200.0),
            }],
        };
        let outputs = CaptureOutputPaths::from_prefix("uart-bench");
        let json = render_capture_metadata_json(&capture, &outputs);
        assert!(json.contains("\"samples_file\": \"uart-bench-ch1.bin\""));
        assert!(!json.contains("\"uart-bench-ch2.bin\""));
    }

    #[test]
    fn resolve_samples_path_uses_metadata_directory() {
        let metadata = Path::new("/tmp/captures/example-meta.json");
        let samples = resolve_samples_path(metadata, "example-ch1.bin");
        assert_eq!(samples, Path::new("/tmp/captures/example-ch1.bin"));
    }

    fn testdata_root() -> &'static Path {
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testdata/uart/amulet_lcd_menu_changes"
        ))
    }
}
