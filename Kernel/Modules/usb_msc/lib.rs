// "Tifflin" Kernel - USB MSC driver
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_msc/lib.rs
//! USB MSC (Mass Storage Class) driver
#![no_std]
#![feature(linkage)]	// for module_define!
use kernel::prelude::*;

#[macro_use]
extern crate kernel;

module_define!{usb_hid, [usb_core, GUI], init}

fn init()
{
	static USB_DRIVER: Driver = Driver;
	::usb_core::device::register_driver(&USB_DRIVER);
}

struct Driver;
impl ::usb_core::device::Driver for Driver
{
	fn name(&self) -> &str {
		"msc"
	}
	fn matches(&self, _vendor_id: u16, _device_id: u16, class_code: u32) -> ::usb_core::device::MatchLevel {
		use ::usb_core::device::MatchLevel;
		// Mass storage with SCSI interface
		if class_code == 0x080650 {
			MatchLevel::Generic
		}
		else {
			MatchLevel::None
		}
	}
	fn start_device<'a>(&self, _ep0: &'a ::usb_core::ControlEndpoint, endpoints: Vec<::usb_core::Endpoint>, _descriptors: &[u8]) -> ::usb_core::device::Instance<'a> {
		let (ep_in, ep_out) = {
			let mut it = endpoints.into_iter();

			let bulk_in = match it.next()
				{
				Some(::usb_core::Endpoint::BulkIn(v)) => v,
				_ => panic!(""),
				};
			let bulk_out = match it.next()
				{
				Some(::usb_core::Endpoint::BulkOut(v)) => v,
				_ => panic!(""),
				};
			(bulk_in, bulk_out,)
			};
		log_notice!("USB MSC");
		// Spin up a SCSI instance.
		let pv_handle = match ::storage_scsi::Volume::new_boxed( ScsiInterface::new(ep_in, ep_out) )
			{
			Ok(scsi_vol) => Some( ::kernel::metadevs::storage::register_pv( scsi_vol ) ),
			Err(e) => {
				log_error!("Error while creating SCSI device: {:?}", e);
				None
				},
			};

		Box::new(DeviceInstance {
			_handle: pv_handle,
			})
	}
}

struct DeviceInstance
{
	_handle: Option<::kernel::metadevs::storage::PhysicalVolumeReg>,
}
impl ::core::future::Future for DeviceInstance
{
	type Output = ();
	fn poll(self: ::core::pin::Pin<&mut Self>, _cx: &mut ::core::task::Context<'_>) -> ::core::task::Poll<()> {
		::core::task::Poll::Pending
	}
}

struct ScsiInterface
{
	name: String,
	//inner: ::kernel::futures::Mutex<ScsiInterfaceInner>,
	inner: ::kernel::sync::Mutex<ScsiInterfaceInner>,
}
impl ScsiInterface
{
	fn new(ep_in: ::usb_core::BulkEndpointIn, ep_out: ::usb_core::BulkEndpointOut) -> Self
	{
		use ::core::sync::atomic::{AtomicUsize,Ordering};
		// TODO: Allow freeing of indexes? Or get the device ID?
		static INDEX: AtomicUsize = AtomicUsize::new(0);
		ScsiInterface {
			name: format!("usb{}", INDEX.fetch_add(1, Ordering::SeqCst)),
			inner: ::kernel::sync::Mutex::new(ScsiInterfaceInner { next_tag: 0, ep_in, ep_out }),
			}
	}
}
impl ::storage_scsi::ScsiInterface for ScsiInterface
{
	fn name(&self) -> &str {
		&self.name
	}
	fn send<'a>(&'a self, command: &[u8], data: &'a [u8]) -> ::kernel::metadevs::storage::AsyncIoResult<'a,()> {
		assert!( command.len() < 16 );
		let len = command.len();
		let bytes = Cbw::slice_to_array(command);
		Box::new( ::kernel::r#async::FutureWrapper::new(async move {
			let mut lh = self.inner.lock();//.await;
			lh.send_data(0, &bytes[..len], data).await;
			Ok( () )
			}) )
	}
	fn recv<'a>(&'a self, command: &[u8], data: &'a mut [u8]) -> ::kernel::metadevs::storage::AsyncIoResult<'a,()>  {
		assert!( command.len() < 16 );
		let len = command.len();
		let bytes = Cbw::slice_to_array(command);
		// TODO: Rewrite kernel async layer to use futures.
		Box::new( ::kernel::r#async::FutureWrapper::new(async move {
			let mut lh = self.inner.lock();//.await;
			lh.recv_data(0, &bytes[..len], data).await;
			Ok( () )
			}) )
	}
}
struct ScsiInterfaceInner
{
	next_tag: u32,
	ep_in: ::usb_core::BulkEndpointIn,
	ep_out: ::usb_core::BulkEndpointOut,
}
impl ScsiInterfaceInner
{
	async fn recv_data(&mut self, lun: u8, cmd: &[u8], buf: &mut [u8])
	{
		let tag = self.next_tag;
		self.next_tag += 1;
		log_debug!("recv_data: {:?} tag={} buf={}", cmd, tag, buf.len());

		// NOTE: Have to do these in sequence, otherwise the controller might schedule the IN before the OUTs

		// Send CBW (including command bytes)
		let cbw = Cbw {
			sig: Cbw::SIG,
			tag: tag,
			data_len: buf.len() as u32,

			flags: Cbw::FLAG_INPUT,
			lun: lun,
			cmd_len: cmd.len() as u8,
			cmd_bytes: Cbw::slice_to_array(cmd),
			};
		let cbw_bytes = cbw.to_bytes();
		self.ep_out.send(&cbw_bytes).await;
		// Receive data (would be nice if this allowed multiple in-flight requests)
		self.ep_in.recv(buf).await;
		// Receive CSW
		let mut csw_bytes = [0; 12+1];
		self.ep_in.recv(&mut csw_bytes).await;
		let csw = Csw::from_bytes(csw_bytes);
		assert!(csw.sig == Csw::SIG, "CSW signature error: {:08x}", csw.sig);
		assert!(csw.tag == tag, "CSW tag mismatch: {} != tag {}", csw.tag, tag);
		log_notice!("recv_data: csw = {:?}", csw);
		// TODO: error return
	}
	async fn send_data(&mut self, lun: u8, cmd: &[u8], buf: &[u8])
	{
		let tag = self.next_tag;
		self.next_tag += 1;
		log_debug!("send_data: {:?} tag={}, buf={:?}", cmd, tag, ::kernel::logging::HexDump(buf));
		// Send CBW (including command bytes)
		let cbw = Cbw {
			sig: Cbw::SIG,
			tag: tag,
			data_len: buf.len() as u32,

			flags: 0,
			lun: lun,
			cmd_len: cmd.len() as u8,
			cmd_bytes: Cbw::slice_to_array(cmd),
			};
		let cbw_bytes = cbw.to_bytes();
		self.ep_out.send(&cbw_bytes).await;
		// Send data
		self.ep_out.send(buf).await;
		// Receive CSW
		let mut csw_bytes = [0; 12+1];
		self.ep_in.recv(&mut csw_bytes).await;
		let csw = Csw::from_bytes(csw_bytes);
		assert!(csw.sig == Csw::SIG, "CSW signature error: {:08x}", csw.sig);
		assert!(csw.tag == tag, "CSW tag mismatch: {} != tag {}", csw.tag, tag);
		if csw.data_residue != 0 {
		}
		log_notice!("send_data: csw = {:?}", csw);
		// TODO: error return
	}
}

#[derive(Debug)]
struct Cbw
{
	pub sig: u32,
	pub tag: u32,
	pub data_len: u32,
	pub flags: u8,
	pub lun: u8,
	pub cmd_len: u8,
	pub cmd_bytes: [u8; 16],
}
impl Cbw
{
	const SIG: u32 = 0x43425355;
	const FLAG_INPUT: u8 = 0x80;
	pub fn to_bytes(self) -> [u8; 12+3+16]
	{
		[
			(self.sig >>  0) as u8, (self.sig >>  8) as u8, (self.sig >> 16) as u8, (self.sig >> 24) as u8,
			(self.tag >>  0) as u8, (self.tag >>  8) as u8, (self.tag >> 16) as u8, (self.tag >> 24) as u8,
			(self.data_len >>  0) as u8, (self.data_len >>  8) as u8, (self.data_len >> 16) as u8, (self.data_len >> 24) as u8,
			self.flags,
			self.lun,
			self.cmd_len,
			self.cmd_bytes[ 0], self.cmd_bytes[ 1], self.cmd_bytes[ 2], self.cmd_bytes[ 3],
			self.cmd_bytes[ 4], self.cmd_bytes[ 5], self.cmd_bytes[ 6], self.cmd_bytes[ 7],
			self.cmd_bytes[ 8], self.cmd_bytes[ 9], self.cmd_bytes[10], self.cmd_bytes[11],
			self.cmd_bytes[12], self.cmd_bytes[13], self.cmd_bytes[14], self.cmd_bytes[15],
			]
	}

	pub fn slice_to_array(s: &[u8]) -> [u8; 16] {
		let mut rv = [0; 16];
		rv[..s.len()].copy_from_slice(s);
		rv
	}
}

#[derive(Debug)]
struct Csw
{
	pub sig: u32,
	pub tag: u32,
	pub data_residue: u32,

	pub status: u8,
}
impl Csw
{
	const SIG: u32 = 0x53425355;	// 'USBS' (little endian)
	pub fn from_bytes(b: [u8; 12+1]) -> Self
	{
		Csw {
			sig: (b[0] as u32) << 0 | (b[1] as u32) << 8 | (b[2] as u32) << 16 | (b[3] as u32) << 24,
			tag: (b[4] as u32) << 0 | (b[5] as u32) << 8 | (b[6] as u32) << 16 | (b[7] as u32) << 24,
			data_residue: (b[8] as u32) << 0 | (b[9] as u32) << 8 | (b[10] as u32) << 16 | (b[11] as u32) << 24,

			status: b[12],
		}
	}
}

