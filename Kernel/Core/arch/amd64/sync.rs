// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/sync.rs
//! Low-level synchronisaion primitives
use core::sync::atomic::{AtomicBool,Ordering};

const TRACE_IF: bool = false;
//const TRACE_IF: bool = true;

/// Lightweight protecting spinlock
pub struct SpinlockInner
{
	#[doc(hidden)]
	lock: AtomicBool,
}

impl SpinlockInner
{
	pub const fn new() -> Self {
		SpinlockInner {
			lock: AtomicBool::new(false),
		}
	}
	
	pub fn try_inner_lock_cpu(&self) -> bool
	{
		//if self.lock.compare_and_swap(0, cpu_num()+1, Ordering::Acquire) == 0
		if self.lock.compare_and_swap(false, true, Ordering::Acquire) == false
		{
			true
		}
		else
		{
			false
		}
	}
	pub fn inner_lock(&self) {
		//while self.lock.compare_and_swap(0, cpu_num()+1, Ordering::Acquire) != 0
		while self.lock.compare_and_swap(false, true, Ordering::Acquire) == true
		{
		}
		::core::sync::atomic::fence(Ordering::Acquire);
	}
	pub fn inner_release(&self) {
		//::arch::puts("Spinlock::release()\n");
		::core::sync::atomic::fence(Ordering::Release);
		self.lock.store(false, Ordering::Release);
	}
}

/// A handle for frozen interrupts
pub struct HeldInterrupts(bool);

///// Handle for a held spinlock that holds interrupts too
//pub struct HeldSpinlockInt<'lock,T:'lock+Send>
//{
//	lock: &'lock Spinlock<T>,
//	irqs: HeldInterrupts,
//}


/// Prevent interrupts from firing until return value is dropped
pub fn hold_interrupts() -> HeldInterrupts
{
	// SAFE: Correct inline assembly
	let if_set = unsafe {
		let flags: u64;
		asm!("pushf; pop {}; cli", out(reg) flags);	// touches stack
		(flags & 0x200) != 0
		};
	
	if TRACE_IF {
		if if_set {
			::arch::puts("hold_interrupts() - IF cleared\n");
		}
		else {
			::arch::puts("hold_interrupts() - IF maintained\n");
		}
	}
	HeldInterrupts(if_set)
}

impl ::core::ops::Drop for HeldInterrupts
{
	fn drop(&mut self)
	{
		if TRACE_IF {
			if self.0 {
				::arch::puts("HeldInterrupts::drop() - IF set\n");
			}
			else {
				::arch::puts("HeldInterrupts::drop() - IF maintained\n");
			}
		}
		
		if self.0 {
			// SAFE: Just re-enables interrupts
			unsafe { asm!("sti", options(nomem, nostack)); }
		}
	}
}

pub unsafe fn stop_interrupts() {
	asm!("cli", options(nomem, nostack));
}
pub unsafe fn start_interrupts() {
	asm!("sti", options(nomem, nostack));
}

// vim: ft=rust

