
pub mod memory {
	pub type PAddr = u64;
	pub type VAddr = usize;
	pub const PAGE_SIZE: usize = 4096;

	pub mod addresses {
		pub fn is_global(_addr: usize) -> bool {
			false
		}

		pub const STACK_SIZE: usize = 5*0x1000;

		pub const USER_END: usize = 0x8000_0000;

		pub const STACKS_BASE: usize = 0;
		pub const STACKS_END : usize = 0;

		pub const HARDWARE_BASE: usize = 0;
		pub const HARDWARE_END : usize = 0;

		pub const HEAP_START: usize = 0;
		pub const HEAP_END : usize = 0;

		pub const BUMP_START: usize = 0;
		pub const BUMP_END  : usize = 0;
	}
	pub mod virt {
		pub struct AddressSpace;
		impl AddressSpace
		{
			pub fn pid0() -> AddressSpace {
				AddressSpace
			}
			pub fn new(_cstart: usize, _cend: usize) -> Result<AddressSpace,()> {
				//#[cfg(feature="native")]
				return Ok(AddressSpace);
				//todo!("AddressSpace::new");
			}
		}

		pub fn post_init() {
		}

		pub unsafe fn temp_map<T>(_pa: super::PAddr)  -> *mut T {
			::core::ptr::null_mut()
		}
		pub unsafe fn temp_unmap<T>(_a: *mut T) {
		}

		pub fn get_phys<T>(_p: *const T) -> ::memory::PAddr {
			0
		}
		pub fn is_reserved<T>(_p: *const T) -> bool {
			true	// NOTE: Assume all memory is valid
		}
		pub fn get_info<T>(_p: *const T) -> Option<(::memory::PAddr,::memory::virt::ProtectionMode)> {
			None
		}

		pub fn is_fixed_alloc(_addr: *const (), _size: usize) -> bool {
			false
		}
		pub unsafe fn fixed_alloc(_p: ::memory::PAddr, _count: usize) -> Option<*mut ()> {
			None
		}

		pub fn can_map_without_alloc(_a: *mut ()) -> bool {
			false
		}

		pub unsafe fn map(_a: *mut (), _p: ::memory::PAddr, _mode: ::memory::virt::ProtectionMode) {
		}
		pub unsafe fn reprotect(_a: *mut (), _mode: ::memory::virt::ProtectionMode) {
		}
		pub unsafe fn unmap(_a: *mut ()) -> Option<::memory::PAddr> {
			None
		}
	}
	pub mod phys {
		pub fn ref_frame(_frame_idx: u64) {
		}
		pub fn deref_frame(_frame_idx: u64) -> u32 {
			1
		}
		pub fn get_multiref_count(_frame_idx: u64) -> u32 {
			0
		}

		pub fn mark_free(_frame_idx: u64) -> bool {
			false
		}
		pub fn mark_used(_frame_idx: u64) {
		}
	}
}
pub mod sync {
	use core::sync::atomic::Ordering;

	#[derive(Default)]
	pub struct SpinlockInner {
		// Stores a std mutex using a manually-managed pointer
		std: ::core::sync::atomic::AtomicPtr<std::sync::Mutex<()>>,
		tid: ::core::sync::atomic::AtomicU32,
		handle: ::core::sync::atomic::AtomicUsize,
	}
	impl SpinlockInner
	{
		pub const fn new() -> Self {
			SpinlockInner {
				std: ::core::sync::atomic::AtomicPtr::new(0 as *mut _),
				tid: ::core::sync::atomic::AtomicU32::new(0),
				handle: ::core::sync::atomic::AtomicUsize::new(0),
			}
		}
		fn get_std(&self) -> &::std::sync::Mutex<()> {
			let p = self.std.load(Ordering::Relaxed);
			let p = if p.is_null() {
					let v = Box::new( ::std::sync::Mutex::new( () ) );
					let p = Box::leak(v) as *mut _;
					let old = self.std.compare_and_swap(::core::ptr::null_mut(), p, Ordering::Relaxed);
					if !old.is_null() {
						// SAFE: Only just created, and not stored
						let _ = unsafe { Box::from_raw(p) };
						old
					}
					else {
						p
					}
				}
				else {
					p
				};
			// SAFE: Valid pointer
			unsafe { &*p }
		}
		pub fn try_inner_lock_cpu(&self) -> bool {
			let lh = match self.get_std().try_lock()
				{
				Ok(v) => v,
				Err(std::sync::TryLockError::WouldBlock) => return false,
				Err(std::sync::TryLockError::Poisoned(e)) => panic!("Poisoned spinlock mutex: {:?}", e),
				};
			self.tid.store(crate::threads::get_thread_id(), Ordering::SeqCst);
			self.handle.store( Box::into_raw(Box::new(lh)) as usize, Ordering::SeqCst );
			return true;
		}
		pub fn inner_lock(&self) {
			let lh = self.get_std().lock().expect("Spinlock");
			self.tid.store(crate::threads::get_thread_id(), Ordering::SeqCst);
			self.handle.store( Box::into_raw(Box::new(lh)) as usize, Ordering::SeqCst );
		}
		pub unsafe fn inner_release(&self) {
			assert!(self.tid.load(Ordering::SeqCst) == crate::threads::get_thread_id());
			let p = self.handle.swap(0, Ordering::SeqCst) as *mut std::sync::MutexGuard<()>;
			assert!(!p.is_null());
			let _ = Box::from_raw(p);
		}
	}

	//struct HeldIntState {
	//	is_held: bool,
	//	count: usize,
	//}
	lazy_static::lazy_static! {
		static ref IRQ_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new( () );
		//static ref IRQ_LOCK_HANDLE: std::sync::Mutex< Option<std::sync::MutexGuard<'static, ()>> > = std::sync::Mutex::new( None );
	}
	pub struct HeldInterrupts(Option<std::sync::MutexGuard<'static, ()>>);

	pub fn hold_interrupts() -> HeldInterrupts {
		//println!("hold_interrupts()");
		// TODO: Implement this with an optional guard and count for the current thread
		//HeldInterrupts(Some(IRQ_LOCK.lock().unwrap()))
		HeldInterrupts(None)
	}
	impl ::core::ops::Drop for HeldInterrupts {
		fn drop(&mut self) {
			//println!("~hold_interrupts()");
		}
	}

	pub unsafe fn stop_interrupts() {
		//todo!("stop_interrupts");
		//*IRQ_LOCK_HANDLE.lock().unwrap() = Some( IRQ_LOCK.lock().unwrap() );
	}
	pub unsafe fn start_interrupts() {
		//*IRQ_LOCK_HANDLE.lock().unwrap() = None;
	}
}
pub mod interrupts {
	#[derive(Debug)]
	pub struct BindError;
	#[derive(Default)]
	pub struct IRQHandle;
	
	pub fn bind_gsi(_gsi: usize, _handler: fn(*const()), _info: *const ()) -> Result<IRQHandle, BindError> {
		todo!("bind_gsi")
	}
}
pub mod boot {
	pub fn get_boot_string() -> &'static str {
		""
	}
	pub fn get_video_mode() -> Option<::metadevs::video::bootvideo::VideoMode> {
		None
	}
	pub fn get_memory_map() -> &'static [::memory::MemoryMapEnt] {
		&[
			::memory::MemoryMapEnt {
				start: 0,
				size: 0 ,
				state: crate::memory::MemoryState::Free,
				domain: 0,
				}
		]
	}
}
pub mod pci {
	pub fn read(_a: u32) -> u32 {
		0
	}
	pub fn write(_a: u32, _v: u32) {
	}
}
pub mod threads {
	use std::sync::Arc;
	use std::cell::RefCell;
	use std::sync::atomic::{Ordering,AtomicBool};
	lazy_static::lazy_static! {
		static ref SWITCH_LOCK: std::sync::Mutex<usize> = std::sync::Mutex::new( 0 );
	}

	#[derive(Debug)]
	struct ThreadLocalState {
		ptr: *mut ::threads::Thread,
		ptr_moved: bool,
		this_state: Option<Arc<StateInner>>,
	}
	thread_local! {
		static THIS_THREAD_STATE: RefCell<ThreadLocalState> = RefCell::new(ThreadLocalState {
			ptr: std::ptr::null_mut(),
			ptr_moved: false,
			this_state: None,
			});
	}
	#[derive(Debug)]
	struct StateInner {
		condvar: std::sync::Condvar,
		complete: AtomicBool,
		running: AtomicBool,
	}
	pub struct State {
		thread_handle: Option<std::thread::JoinHandle<()>>,
		inner: Arc<StateInner>,
	}
	impl State
	{
		fn new_priv() -> State {
			State {
				thread_handle: None,
				inner: Arc::new(StateInner {
					condvar: Default::default(),
					complete: Default::default(),
					running: Default::default(),
				})
			}
		}
		pub fn new(_as: &::arch::memory::virt::AddressSpace) -> State {
			Self::new_priv()
		}
	}
	impl StateInner
	{
		fn sleep(&self, t: Option<::threads::ThreadPtr>) {
			let mut lh = SWITCH_LOCK.lock().unwrap();
			if let Some(ref t) = t
			{
				t.cpu_state.inner.running.store(true, Ordering::SeqCst);	// Avoids a startup race
				*lh = &*t.cpu_state.inner as *const _ as usize;
				t.cpu_state.inner.condvar.notify_one();
			}
			// TODO: State enum? (PreStart, Running, Sleeping, Paused, Dead)
			if !self.complete.load(Ordering::SeqCst) {
				while *lh != self as *const _ as usize
				{
					log_trace!("{:p} sleeping (current {:#x})", self, *lh);
					lh = self.condvar.wait(lh).expect("Condvar wait failed");
					log_trace!("{:p} awake (current {:#x})", self, *lh);
				}
			}
			else {
				//log_trace!("{:p} complete", self);
			}
			core::mem::forget(t);
			drop(lh);
		}
	}

	pub fn init_tid0_state() -> State {
		let rv = State::new_priv();
		let inner_handle = rv.inner.clone();
		log_trace!("init_tid0_state: {:p}", inner_handle);
		THIS_THREAD_STATE.with(|v| {
			let mut h = v.borrow_mut();
			assert!(h.this_state.is_none(), "TID0 alread initialised");
			h.this_state = Some(inner_handle);
			});
		rv
	}
	pub fn set_thread_ptr(t: ::threads::ThreadPtr) {
		THIS_THREAD_STATE.with(|v| {
			log_trace!("set_thread_ptr");
			let mut h = v.borrow_mut();
			let t: *mut _ = t.unwrap();
			if h.ptr.is_null() {
				h.ptr = t;
			}
			else {
				assert!(h.ptr == t);
				assert!(h.ptr_moved == true);
				h.ptr_moved = false;
			}
		})
	}
	pub fn get_thread_ptr() -> Option<::threads::ThreadPtr> {
		THIS_THREAD_STATE.with(|v| {
			//log_trace!("get_thread_ptr: {:p}", v);
			let mut h = v.borrow_mut();
			assert!(!h.ptr_moved);
			if h.ptr.is_null() {
				None
			}
			else {
				h.ptr_moved = true;
				// SAFE: Pointer to pointer
				Some(unsafe { std::mem::transmute(h.ptr) })
			}
		})
	}
	pub fn borrow_thread() -> *const ::threads::Thread {
		THIS_THREAD_STATE.with(|v| {
			let h = v.borrow();
			// NOTE: Doesn't care if the pointer is "owned"
			h.ptr
		})
	}

	pub fn idle() {
		// Timed sleep?
		std::thread::sleep(std::time::Duration::from_millis(50));
	}
	pub fn get_idle_thread() -> ::threads::ThreadPtr {
		lazy_static::lazy_static! {
			static ref TS_ZERO: usize = crate::threads::new_idle_thread(0).into_usize();
		}
		// SAFE: Same as `get_thread_ptr`, doesn't actually own the result
		unsafe { std::mem::transmute(*TS_ZERO) }
	}
	pub fn switch_to(t: ::threads::ThreadPtr) {
		THIS_THREAD_STATE.with(|v| {
			let h = v.borrow();
			//assert!( h.ptr_moved );
			match h.this_state
			{
			None => panic!("Current thread not initialised"),
			Some(ref v) => {
				log_trace!("switch_to: {:p} to {:p}", *v, t.cpu_state.inner);
				v.sleep(Some(t));
				//log_trace!("switch_to: {:p} awake", *v);
				},
			}
		});
		THIS_THREAD_STATE.with(|v| {
			let mut h = v.borrow_mut();
			//assert!(h.ptr_moved);
			h.ptr_moved = false;
		});
	}

	/// Test hack: Releases the current thread from scheduling
	pub fn test_unlock_thread() {
		// - Hold switching lock until function returns
		// Mark as complete to cause the thread to not sleep on next yield
		THIS_THREAD_STATE.with(|v| {
			let h = v.borrow();
			h.this_state.as_ref().unwrap().complete.store(true, Ordering::SeqCst);
			});
		let lock = crate::sync::RwLock::new( () );
		log_notice!("Thread no longer in scheduling flow");
		// Acquire a lock
		std::mem::forget( lock.write() );
		// - Trigger a deadlock (which will sleep, but not block due to above flag)
		std::mem::forget( lock.read() );
		// Forget the lock to completely forget the thread
		std::mem::forget( lock );
	}

	pub struct ThreadPauser {
		lock: crate::sync::RwLock<()>,
	}
	pub struct PausedThread<'a> {
		wl: Option<crate::sync::rwlock::Write<'a, ()>>,
		rl: Option<crate::sync::rwlock::Read<'a, ()>>,
	}
	impl ThreadPauser {
		pub fn new() -> Self {
			ThreadPauser {
				lock: crate::sync::RwLock::new( () ),
			}
		}
		pub fn pause(&self) -> PausedThread {
			// - Hold switching lock until function returns
			// Mark as complete to cause the thread to not sleep on next yield
			THIS_THREAD_STATE.with(|v| {
				let h = v.borrow();
				h.this_state.as_ref().unwrap().complete.store(true, Ordering::SeqCst);
				});
			log_debug!("Pausing thread");
			// Acquire a lock
			let wl = self.lock.write();
			// - Trigger a deadlock (which will sleep, but not block due to above flag)
			let rl = self.lock.read();

			log_debug!("Paused thread");
			PausedThread {
				wl: Some(wl),
				rl: Some(rl),
			}
		}
	}
	impl ::core::ops::Drop for PausedThread<'_> {
		fn drop(&mut self)
		{
			log_debug!("Unpausing thread");
	
			// Clear `complete`
			let p = THIS_THREAD_STATE.with(|v| {
				let h = v.borrow();
				let state = h.this_state.as_ref().unwrap();
				state.complete.store(false, Ordering::SeqCst);
				state as *const _
				});
			// Release the write lock (will wake with the read lock)
			drop(self.wl.take());
			// And then release the read lock
			drop(self.rl.take());
			log_debug!("Unpaused thread {:p}", p);
	
			// Wait until scheduled
			// TODO: Only wait if not the current thread
			THIS_THREAD_STATE.with(|v| {
				let h = v.borrow();
				//assert!( h.ptr_moved );
				match h.this_state
				{
				None => panic!("Current thread not initialised"),
				Some(ref v) => {
					//log_trace!("switch_to: {:p} to {:p}", *v, t.cpu_state.inner);
					v.sleep(None);
					log_trace!("test_pause_thread: {:p} awake", *v);
					},
				}
			});
		}
	}

	/// Test hack: Take the current thread out of the kernel's scheduling while running a closure
	///
	/// Useful for running a native function that will block for a significant period
	pub fn test_pause_thread<F,T>(f: F) -> T
	where
		F: FnOnce()->T
	{
		let pauser = ThreadPauser::new();

		let handle = pauser.pause();

		let rv = f();
		drop(handle);
		rv
	}

	pub fn start_thread<F: FnOnce()+Send+'static>(thread: &mut ::threads::Thread, code: F) {
		// Set thread state's join handle to a thread with a pause point
		let inner_handle = thread.cpu_state.inner.clone();
		log_trace!("start_thread: {:p}", inner_handle);
		let name = std::format!("{:p}", inner_handle);
		let ptr = thread as *mut _ as usize;
		let th = std::thread::Builder::new()
			.name(name)
			.spawn(move || {
				// Initialise the thread-local structures
				THIS_THREAD_STATE.with(|v| {
					let mut h = v.borrow_mut();
					h.ptr = ptr as *mut _;
					h.this_state = Some(inner_handle.clone());
					});
				// Wait for the first yield
				let mut lh = SWITCH_LOCK.lock().unwrap();
				if ! inner_handle.running.load(Ordering::SeqCst) {
					lh = inner_handle.condvar.wait(lh).expect("Condvar wait failed");
				}
				drop(lh);
				// Run "user" code
				log_trace!("Thread started");
				(code)();
				log_trace!("Thread complete");
				// Mark the thread as being complete
				inner_handle.complete.store(true, Ordering::SeqCst);
				// Yield (which will start the next thread)
				crate::threads::yield_time();
				})
			.unwrap()
			;
		thread.cpu_state.thread_handle = Some(th);
	}
}
pub mod x86_io {
	pub unsafe fn inb(_p: u16) -> u8 { 0 }
	pub unsafe fn inw(_p: u16) -> u16 { 0 }
	pub unsafe fn inl(_p: u16) -> u32 { 0 }
	pub unsafe fn outb(_p: u16, _v: u8) { }
	pub unsafe fn outw(_p: u16, _v: u16) { }
	pub unsafe fn outl(_p: u16, _v: u32) { }
}

pub unsafe fn drop_to_user(_entry: usize, _stack: usize, _args_len: usize) -> ! {
	panic!("todo: drop_to_user");
}
pub fn puts(s: &str) {
	print!("{}", s);
}
pub fn puth(v: u64) {
	print!("{:08x}", v);
}
pub fn cur_timestamp() -> u64 {
	lazy_static::lazy_static! {
		static ref TS_ZERO: std::time::Instant = std::time::Instant::now();
	}
	let ts0 = *TS_ZERO;
	(std::time::Instant::now() - ts0).as_millis() as u64
}
pub fn print_backtrace() {
}

