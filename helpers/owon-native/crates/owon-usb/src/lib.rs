use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusb::UsbContext;

pub const OWON_VENDOR_ID: u16 = 0x5345;
pub const OWON_PRODUCT_ID: u16 = 0x1234;
pub const OWON_INTERFACE_NUMBER: u8 = 0;

#[derive(Debug)]
pub enum UsbProbeError {
    Usb(rusb::Error),
    Io(io::Error),
}

impl fmt::Display for UsbProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usb(error) => write!(f, "USB error: {error}"),
            Self::Io(error) => write!(f, "I/O error: {error}"),
        }
    }
}

impl Error for UsbProbeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Usb(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<rusb::Error> for UsbProbeError {
    fn from(value: rusb::Error) -> Self {
        Self::Usb(value)
    }
}

impl From<io::Error> for UsbProbeError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeBackend {
    Rusb,
    Sysfs,
}

impl fmt::Display for ProbeBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rusb => write!(f, "rusb"),
            Self::Sysfs => write!(f, "sysfs"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointDirection {
    In,
    Out,
}

impl fmt::Display for EndpointDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::In => write!(f, "IN"),
            Self::Out => write!(f, "OUT"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointTransferType {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
}

impl fmt::Display for EndpointTransferType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Control => write!(f, "Control"),
            Self::Isochronous => write!(f, "Isochronous"),
            Self::Bulk => write!(f, "Bulk"),
            Self::Interrupt => write!(f, "Interrupt"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointInfo {
    pub address: u8,
    pub direction: EndpointDirection,
    pub transfer_type: EndpointTransferType,
    pub max_packet_size: u16,
    pub interval: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceSnapshot {
    pub number: u8,
    pub alternate_setting: u8,
    pub class_code: u8,
    pub sub_class_code: u8,
    pub protocol_code: u8,
    pub driver: Option<String>,
    pub modalias: Option<String>,
    pub endpoints: Vec<EndpointInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSnapshot {
    pub backend: ProbeBackend,
    pub bus_number: u8,
    pub address: u8,
    pub sysfs_name: Option<String>,
    pub sysfs_path: Option<PathBuf>,
    pub interface_sysfs_name: Option<String>,
    pub interface_sysfs_path: Option<PathBuf>,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_class: u8,
    pub device_sub_class: u8,
    pub device_protocol: u8,
    pub max_packet_size_0: u8,
    pub num_configurations: u8,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub speed_mbps: Option<String>,
    pub usb_version: Option<String>,
    pub device_release: Option<String>,
    pub interfaces: Vec<InterfaceSnapshot>,
}

impl DeviceSnapshot {
    pub fn bus_device_path(&self) -> String {
        format!("bus/usb/{:03}/{:03}", self.bus_number, self.address)
    }

    pub fn device_node_path(&self) -> PathBuf {
        device_node_path(self.bus_number, self.address)
    }

    pub fn primary_interface(&self) -> Option<&InterfaceSnapshot> {
        self.interfaces
            .iter()
            .find(|interface| interface.number == OWON_INTERFACE_NUMBER)
    }

    pub fn active_driver(&self) -> Option<&str> {
        self.primary_interface()
            .and_then(|interface| interface.driver.as_deref())
    }

    pub fn matches_expected_layout(&self) -> bool {
        let Some(interface) = self.primary_interface() else {
            return false;
        };

        if self.interfaces.len() != 1 {
            return false;
        }

        if interface.alternate_setting != 0 || interface.endpoints.len() != 2 {
            return false;
        }

        let has_bulk_in = interface.endpoints.iter().any(|endpoint| {
            endpoint.direction == EndpointDirection::In
                && endpoint.transfer_type == EndpointTransferType::Bulk
                && endpoint.max_packet_size == 64
        });
        let has_bulk_out = interface.endpoints.iter().any(|endpoint| {
            endpoint.direction == EndpointDirection::Out
                && endpoint.transfer_type == EndpointTransferType::Bulk
                && endpoint.max_packet_size == 64
        });

        has_bulk_in && has_bulk_out
    }
}

#[derive(Debug)]
pub enum UsbOpenError {
    Probe(UsbProbeError),
    DeviceNotFound,
    DeviceNodeMissing(PathBuf),
    UnexpectedLayout,
    RusbEnumeration(rusb::Error),
    DeviceDescriptor(rusb::Error),
    OpenHandle(rusb::Error),
    KernelDriverQuery(rusb::Error),
    KernelDriverDetachNotSupported,
    KernelDriverDetach(rusb::Error),
    ClaimInterface(rusb::Error),
    ReleaseInterface(rusb::Error),
    ReattachKernelDriver(rusb::Error),
}

impl fmt::Display for UsbOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Probe(error) => write!(f, "{error}"),
            Self::DeviceNotFound => write!(f, "No OWON VDS1022 devices found"),
            Self::DeviceNodeMissing(path) => write!(
                f,
                "USB device node is missing: {}. Direct USB access is unavailable in this environment.",
                path.display()
            ),
            Self::UnexpectedLayout => write!(
                f,
                "Device descriptor layout does not match the expected OWON shape"
            ),
            Self::RusbEnumeration(error) => {
                write!(f, "Failed to enumerate USB devices with rusb: {error}")
            }
            Self::DeviceDescriptor(error) => {
                write!(f, "Failed to read USB device descriptor: {error}")
            }
            Self::OpenHandle(error) => {
                if matches!(error, rusb::Error::Access) {
                    write!(
                        f,
                        "Failed to open USB device handle: {error}. Install the helper-pack udev rule with `sudo bash scripts/install-udev-rule.sh`, or run the open test with root privileges."
                    )
                } else {
                    write!(f, "Failed to open USB device handle: {error}")
                }
            }
            Self::KernelDriverQuery(error) => {
                write!(f, "Failed to query active kernel driver state: {error}")
            }
            Self::KernelDriverDetachNotSupported => write!(
                f,
                "Kernel driver detach is not supported by this libusb setup"
            ),
            Self::KernelDriverDetach(error) => {
                write!(f, "Failed to detach active kernel driver: {error}")
            }
            Self::ClaimInterface(error) => write!(f, "Failed to claim interface 0: {error}"),
            Self::ReleaseInterface(error) => write!(f, "Failed to release interface 0: {error}"),
            Self::ReattachKernelDriver(error) => write!(
                f,
                "Failed to reattach kernel driver after open test: {error}"
            ),
        }
    }
}

impl Error for UsbOpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Probe(error) => Some(error),
            Self::RusbEnumeration(error)
            | Self::DeviceDescriptor(error)
            | Self::OpenHandle(error)
            | Self::KernelDriverQuery(error)
            | Self::KernelDriverDetach(error)
            | Self::ClaimInterface(error)
            | Self::ReleaseInterface(error)
            | Self::ReattachKernelDriver(error) => Some(error),
            Self::DeviceNotFound
            | Self::DeviceNodeMissing(_)
            | Self::UnexpectedLayout
            | Self::KernelDriverDetachNotSupported => None,
        }
    }
}

impl From<UsbProbeError> for UsbOpenError {
    fn from(value: UsbProbeError) -> Self {
        Self::Probe(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenReport {
    pub snapshot: DeviceSnapshot,
    pub device_node_path: PathBuf,
    pub detach_supported: bool,
    pub kernel_driver_was_active: bool,
    pub detached_kernel_driver: bool,
    pub claimed_interface: u8,
    pub released_interface: bool,
    pub reattached_kernel_driver: bool,
}

#[derive(Debug)]
pub enum UsbTransferError {
    Write(rusb::Error),
    ShortWrite { expected: usize, actual: usize },
    Read(rusb::Error),
    ShortRead { expected: usize, actual: usize },
}

impl fmt::Display for UsbTransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Write(error) => write!(f, "USB bulk write failed: {error}"),
            Self::ShortWrite { expected, actual } => {
                write!(
                    f,
                    "USB bulk write was short: expected {expected} bytes, wrote {actual}"
                )
            }
            Self::Read(error) => write!(f, "USB bulk read failed: {error}"),
            Self::ShortRead { expected, actual } => {
                write!(
                    f,
                    "USB bulk read was short: expected {expected} bytes, got {actual}"
                )
            }
        }
    }
}

impl Error for UsbTransferError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Write(error) | Self::Read(error) => Some(error),
            Self::ShortWrite { .. } | Self::ShortRead { .. } => None,
        }
    }
}

impl UsbTransferError {
    pub fn is_read_timeout(&self) -> bool {
        matches!(self, Self::Read(rusb::Error::Timeout))
    }
}

pub const DEFAULT_USB_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Debug)]
pub struct OpenSession {
    snapshot: DeviceSnapshot,
    handle: rusb::DeviceHandle<rusb::Context>,
    read_endpoint: u8,
    write_endpoint: u8,
    detach_supported: bool,
    kernel_driver_was_active: bool,
    detached_kernel_driver: bool,
    closed: bool,
}

impl OpenSession {
    pub fn snapshot(&self) -> &DeviceSnapshot {
        &self.snapshot
    }

    pub fn read_endpoint(&self) -> u8 {
        self.read_endpoint
    }

    pub fn write_endpoint(&self) -> u8 {
        self.write_endpoint
    }

    pub fn detach_supported(&self) -> bool {
        self.detach_supported
    }

    pub fn kernel_driver_was_active(&self) -> bool {
        self.kernel_driver_was_active
    }

    pub fn detached_kernel_driver(&self) -> bool {
        self.detached_kernel_driver
    }

    pub fn write_bulk_exact(
        &self,
        buffer: &[u8],
        timeout: Duration,
    ) -> Result<(), UsbTransferError> {
        let written = self
            .handle
            .write_bulk(self.write_endpoint, buffer, timeout)
            .map_err(UsbTransferError::Write)?;
        if written != buffer.len() {
            return Err(UsbTransferError::ShortWrite {
                expected: buffer.len(),
                actual: written,
            });
        }
        Ok(())
    }

    pub fn read_bulk_exact(
        &self,
        buffer: &mut [u8],
        timeout: Duration,
    ) -> Result<(), UsbTransferError> {
        let read = self.read_bulk(buffer, timeout)?;
        if read != buffer.len() {
            return Err(UsbTransferError::ShortRead {
                expected: buffer.len(),
                actual: read,
            });
        }
        Ok(())
    }

    pub fn read_bulk(
        &self,
        buffer: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, UsbTransferError> {
        self.handle
            .read_bulk(self.read_endpoint, buffer, timeout)
            .map_err(UsbTransferError::Read)
    }

    pub fn close(mut self) -> Result<(), UsbOpenError> {
        self.close_internal()?;
        self.closed = true;
        Ok(())
    }

    fn close_internal(&mut self) -> Result<(), UsbOpenError> {
        self.handle
            .release_interface(OWON_INTERFACE_NUMBER)
            .map_err(UsbOpenError::ReleaseInterface)?;

        if self.detached_kernel_driver {
            self.handle
                .attach_kernel_driver(OWON_INTERFACE_NUMBER)
                .map_err(UsbOpenError::ReattachKernelDriver)?;
        }

        Ok(())
    }
}

impl Drop for OpenSession {
    fn drop(&mut self) {
        if self.closed {
            return;
        }

        let _ = self.handle.release_interface(OWON_INTERFACE_NUMBER);
        if self.detached_kernel_driver {
            let _ = self.handle.attach_kernel_driver(OWON_INTERFACE_NUMBER);
        }
        self.closed = true;
    }
}

pub fn open_session() -> Result<OpenSession, UsbOpenError> {
    let snapshot = scan_vds1022_devices()?
        .into_iter()
        .next()
        .ok_or(UsbOpenError::DeviceNotFound)?;
    if !snapshot.matches_expected_layout() {
        return Err(UsbOpenError::UnexpectedLayout);
    }

    let device_node_path = device_node_path(snapshot.bus_number, snapshot.address);
    if !device_node_path.exists() {
        return Err(UsbOpenError::DeviceNodeMissing(device_node_path));
    }

    let context = rusb::Context::new().map_err(UsbOpenError::RusbEnumeration)?;
    let devices = context.devices().map_err(UsbOpenError::RusbEnumeration)?;
    let mut matched_device = None;

    for device in devices.iter() {
        if device.bus_number() != snapshot.bus_number || device.address() != snapshot.address {
            continue;
        }

        let descriptor = device
            .device_descriptor()
            .map_err(UsbOpenError::DeviceDescriptor)?;
        if descriptor.vendor_id() == snapshot.vendor_id
            && descriptor.product_id() == snapshot.product_id
        {
            matched_device = Some(device);
            break;
        }
    }

    let Some(device) = matched_device else {
        return Err(UsbOpenError::DeviceNotFound);
    };

    let handle = device.open().map_err(UsbOpenError::OpenHandle)?;
    let detach_supported = rusb::supports_detach_kernel_driver();
    let kernel_driver_was_active = handle
        .kernel_driver_active(OWON_INTERFACE_NUMBER)
        .map_err(UsbOpenError::KernelDriverQuery)?;
    let mut detached_kernel_driver = false;
    if kernel_driver_was_active {
        if !detach_supported {
            return Err(UsbOpenError::KernelDriverDetachNotSupported);
        }
        handle
            .detach_kernel_driver(OWON_INTERFACE_NUMBER)
            .map_err(UsbOpenError::KernelDriverDetach)?;
        detached_kernel_driver = true;
    }

    if let Err(error) = handle.claim_interface(OWON_INTERFACE_NUMBER) {
        if detached_kernel_driver {
            let _ = handle.attach_kernel_driver(OWON_INTERFACE_NUMBER);
        }
        return Err(UsbOpenError::ClaimInterface(error));
    }

    let interface = snapshot
        .primary_interface()
        .ok_or(UsbOpenError::UnexpectedLayout)?;
    let read_endpoint = interface
        .endpoints
        .iter()
        .find(|endpoint| endpoint.direction == EndpointDirection::In)
        .map(|endpoint| endpoint.address)
        .ok_or(UsbOpenError::UnexpectedLayout)?;
    let write_endpoint = interface
        .endpoints
        .iter()
        .find(|endpoint| endpoint.direction == EndpointDirection::Out)
        .map(|endpoint| endpoint.address)
        .ok_or(UsbOpenError::UnexpectedLayout)?;

    Ok(OpenSession {
        snapshot,
        handle,
        read_endpoint,
        write_endpoint,
        detach_supported,
        kernel_driver_was_active,
        detached_kernel_driver,
        closed: false,
    })
}

pub fn open_vds1022() -> Result<OpenReport, UsbOpenError> {
    let session = open_session()?;
    let report = OpenReport {
        snapshot: session.snapshot.clone(),
        device_node_path: session.snapshot.device_node_path(),
        detach_supported: session.detach_supported,
        kernel_driver_was_active: session.kernel_driver_was_active,
        detached_kernel_driver: session.detached_kernel_driver,
        claimed_interface: OWON_INTERFACE_NUMBER,
        released_interface: true,
        reattached_kernel_driver: session.detached_kernel_driver,
    };
    session.close()?;

    Ok(report)
}

pub fn scan_vds1022_devices() -> Result<Vec<DeviceSnapshot>, UsbProbeError> {
    match scan_vds1022_devices_via_rusb() {
        Ok(devices) => Ok(devices),
        Err(UsbProbeError::Usb(_)) => scan_vds1022_devices_from_sysfs(),
        Err(error) => Err(error),
    }
}

pub fn scan_vds1022_devices_via_rusb() -> Result<Vec<DeviceSnapshot>, UsbProbeError> {
    let context = rusb::Context::new()?;
    let devices = context.devices()?;
    let mut snapshots = Vec::new();

    for device in devices.iter() {
        let descriptor = device.device_descriptor()?;
        if descriptor.vendor_id() != OWON_VENDOR_ID || descriptor.product_id() != OWON_PRODUCT_ID {
            continue;
        }

        snapshots.push(snapshot_device(&device, &descriptor)?);
    }

    Ok(snapshots)
}

pub fn scan_vds1022_devices_from_sysfs() -> Result<Vec<DeviceSnapshot>, UsbProbeError> {
    let base = Path::new("/sys/bus/usb/devices");
    let mut snapshots = Vec::new();

    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.contains(':') {
            continue;
        }

        let path = entry.path();
        let Some(vendor_id) = read_hex_u16(path.join("idVendor")) else {
            continue;
        };
        let Some(product_id) = read_hex_u16(path.join("idProduct")) else {
            continue;
        };
        if vendor_id != OWON_VENDOR_ID || product_id != OWON_PRODUCT_ID {
            continue;
        }

        snapshots.push(snapshot_device_from_sysfs(path, name)?);
    }

    snapshots.sort_by_key(|device| (device.bus_number, device.address));
    Ok(snapshots)
}

fn snapshot_device(
    device: &rusb::Device<rusb::Context>,
    descriptor: &rusb::DeviceDescriptor,
) -> Result<DeviceSnapshot, UsbProbeError> {
    let sysfs_name = sysfs_name(device);
    let sysfs_path = sysfs_name
        .as_ref()
        .map(|name| PathBuf::from("/sys/bus/usb/devices").join(name));
    let interface_sysfs_name = sysfs_name
        .as_ref()
        .map(|name| format!("{name}:1.{OWON_INTERFACE_NUMBER}"));
    let interface_sysfs_path = interface_sysfs_name
        .as_ref()
        .map(|name| PathBuf::from("/sys/bus/usb/devices").join(name));

    let config = device
        .active_config_descriptor()
        .or_else(|_| device.config_descriptor(0))?;
    let interfaces = config
        .interfaces()
        .flat_map(|interface| interface.descriptors())
        .map(|descriptor| snapshot_interface(sysfs_name.as_deref(), &descriptor))
        .collect::<Vec<_>>();

    Ok(DeviceSnapshot {
        backend: ProbeBackend::Rusb,
        bus_number: device.bus_number(),
        address: device.address(),
        sysfs_name,
        sysfs_path: sysfs_path.clone(),
        interface_sysfs_name,
        interface_sysfs_path,
        vendor_id: descriptor.vendor_id(),
        product_id: descriptor.product_id(),
        device_class: descriptor.class_code(),
        device_sub_class: descriptor.sub_class_code(),
        device_protocol: descriptor.protocol_code(),
        max_packet_size_0: descriptor.max_packet_size(),
        num_configurations: descriptor.num_configurations(),
        manufacturer: sysfs_path
            .as_ref()
            .and_then(|path| read_trimmed(path.join("manufacturer"))),
        product: sysfs_path
            .as_ref()
            .and_then(|path| read_trimmed(path.join("product"))),
        serial: sysfs_path
            .as_ref()
            .and_then(|path| read_trimmed(path.join("serial"))),
        speed_mbps: sysfs_path
            .as_ref()
            .and_then(|path| read_trimmed(path.join("speed"))),
        usb_version: sysfs_path
            .as_ref()
            .and_then(|path| read_trimmed(path.join("version"))),
        device_release: sysfs_path
            .as_ref()
            .and_then(|path| read_trimmed(path.join("bcdDevice"))),
        interfaces,
    })
}

fn snapshot_device_from_sysfs(
    path: PathBuf,
    sysfs_name: String,
) -> Result<DeviceSnapshot, UsbProbeError> {
    let interfaces = parse_interfaces_from_sysfs(&path, &sysfs_name)?;
    let interface_sysfs_name = interfaces
        .iter()
        .find(|interface| interface.number == OWON_INTERFACE_NUMBER)
        .map(|interface| format!("{sysfs_name}:1.{}", interface.number));
    let interface_sysfs_path = interface_sysfs_name
        .as_ref()
        .map(|name| PathBuf::from("/sys/bus/usb/devices").join(name));

    Ok(DeviceSnapshot {
        backend: ProbeBackend::Sysfs,
        bus_number: read_decimal_u8(path.join("busnum")).unwrap_or(0),
        address: read_decimal_u8(path.join("devnum")).unwrap_or(0),
        sysfs_name: Some(sysfs_name),
        sysfs_path: Some(path.clone()),
        interface_sysfs_name,
        interface_sysfs_path,
        vendor_id: read_hex_u16(path.join("idVendor")).unwrap_or(0),
        product_id: read_hex_u16(path.join("idProduct")).unwrap_or(0),
        device_class: read_hex_u8(path.join("bDeviceClass")).unwrap_or(0),
        device_sub_class: read_hex_u8(path.join("bDeviceSubClass")).unwrap_or(0),
        device_protocol: read_hex_u8(path.join("bDeviceProtocol")).unwrap_or(0),
        max_packet_size_0: read_decimal_u8(path.join("bMaxPacketSize0")).unwrap_or(0),
        num_configurations: read_decimal_u8(path.join("bNumConfigurations")).unwrap_or(0),
        manufacturer: read_trimmed(path.join("manufacturer")),
        product: read_trimmed(path.join("product")),
        serial: read_trimmed(path.join("serial")),
        speed_mbps: read_trimmed(path.join("speed")),
        usb_version: read_trimmed(path.join("version")),
        device_release: read_trimmed(path.join("bcdDevice")),
        interfaces,
    })
}

fn snapshot_interface(
    sysfs_name: Option<&str>,
    descriptor: &rusb::InterfaceDescriptor<'_>,
) -> InterfaceSnapshot {
    let interface_sysfs_path = sysfs_name.map(|name| {
        PathBuf::from("/sys/bus/usb/devices")
            .join(format!("{name}:1.{}", descriptor.interface_number()))
    });
    let driver = interface_sysfs_path
        .as_ref()
        .and_then(|path| read_link_basename(path.join("driver")));
    let modalias = interface_sysfs_path
        .as_ref()
        .and_then(|path| read_trimmed(path.join("modalias")));
    let endpoints = descriptor
        .endpoint_descriptors()
        .map(|endpoint| EndpointInfo {
            address: endpoint.address(),
            direction: match endpoint.direction() {
                rusb::Direction::In => EndpointDirection::In,
                rusb::Direction::Out => EndpointDirection::Out,
            },
            transfer_type: match endpoint.transfer_type() {
                rusb::TransferType::Control => EndpointTransferType::Control,
                rusb::TransferType::Isochronous => EndpointTransferType::Isochronous,
                rusb::TransferType::Bulk => EndpointTransferType::Bulk,
                rusb::TransferType::Interrupt => EndpointTransferType::Interrupt,
            },
            max_packet_size: endpoint.max_packet_size(),
            interval: endpoint.interval(),
        })
        .collect();

    InterfaceSnapshot {
        number: descriptor.interface_number(),
        alternate_setting: descriptor.setting_number(),
        class_code: descriptor.class_code(),
        sub_class_code: descriptor.sub_class_code(),
        protocol_code: descriptor.protocol_code(),
        driver,
        modalias,
        endpoints,
    }
}

fn parse_interfaces_from_sysfs(
    device_path: &Path,
    device_name: &str,
) -> Result<Vec<InterfaceSnapshot>, UsbProbeError> {
    let raw = fs::read(device_path.join("descriptors"))?;
    let mut interfaces = parse_interface_descriptors(&raw);

    for interface in &mut interfaces {
        let interface_name = format!("{device_name}:1.{}", interface.number);
        let interface_path = PathBuf::from("/sys/bus/usb/devices").join(interface_name);
        interface.driver = read_link_basename(interface_path.join("driver"));
        interface.modalias = read_trimmed(interface_path.join("modalias"));
    }

    Ok(interfaces)
}

fn parse_interface_descriptors(raw: &[u8]) -> Vec<InterfaceSnapshot> {
    let mut interfaces = Vec::new();
    let mut current_interface = None;
    let mut offset = 0usize;

    while offset + 1 < raw.len() {
        let length = raw[offset] as usize;
        if length == 0 || offset + length > raw.len() {
            break;
        }

        match raw[offset + 1] {
            0x04 if length >= 9 => {
                interfaces.push(InterfaceSnapshot {
                    number: raw[offset + 2],
                    alternate_setting: raw[offset + 3],
                    class_code: raw[offset + 5],
                    sub_class_code: raw[offset + 6],
                    protocol_code: raw[offset + 7],
                    driver: None,
                    modalias: None,
                    endpoints: Vec::new(),
                });
                current_interface = Some(interfaces.len() - 1);
            }
            0x05 if length >= 7 => {
                if let Some(index) = current_interface {
                    interfaces[index].endpoints.push(EndpointInfo {
                        address: raw[offset + 2],
                        direction: if raw[offset + 2] & 0x80 != 0 {
                            EndpointDirection::In
                        } else {
                            EndpointDirection::Out
                        },
                        transfer_type: match raw[offset + 3] & 0x03 {
                            0 => EndpointTransferType::Control,
                            1 => EndpointTransferType::Isochronous,
                            2 => EndpointTransferType::Bulk,
                            _ => EndpointTransferType::Interrupt,
                        },
                        max_packet_size: u16::from_le_bytes([raw[offset + 4], raw[offset + 5]]),
                        interval: raw[offset + 6],
                    });
                }
            }
            _ => {}
        }

        offset += length;
    }

    interfaces
}

fn sysfs_name(device: &rusb::Device<rusb::Context>) -> Option<String> {
    let ports = device.port_numbers().ok()?;
    if ports.is_empty() {
        return None;
    }

    let joined_ports = ports
        .into_iter()
        .map(|port| port.to_string())
        .collect::<Vec<_>>()
        .join(".");
    Some(format!("{}-{joined_ports}", device.bus_number()))
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn read_link_basename(path: impl AsRef<Path>) -> Option<String> {
    let target = fs::read_link(path).ok()?;
    target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn read_decimal_u8(path: impl AsRef<Path>) -> Option<u8> {
    let raw = read_trimmed(path)?;
    raw.parse().ok()
}

fn read_hex_u8(path: impl AsRef<Path>) -> Option<u8> {
    let raw = read_trimmed(path)?;
    u8::from_str_radix(&raw, 16).ok()
}

fn read_hex_u16(path: impl AsRef<Path>) -> Option<u16> {
    let raw = read_trimmed(path)?;
    u16::from_str_radix(&raw, 16).ok()
}

fn device_node_path(bus_number: u8, address: u8) -> PathBuf {
    PathBuf::from(format!("/dev/bus/usb/{bus_number:03}/{address:03}"))
}
