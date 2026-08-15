//! IRQ-disabling lock policies.
//!
//! This module provides lock wrappers that disable hardware interrupts (IRQs)
//! upon acquisition and restore their previous state upon release. This is
//! essential for kernel development or bare-metal embedded systems where an
//! interrupt handler might attempt to acquire the same lock held by the
//! interrupted thread, leading to a deadlock.

use core::convert::Infallible;

use crate::LockStatus;
use crate::traits::{LockPolicy, SharingPolicy};

/// Disables IRQs and returns a boolean indicating whether interrupts were
/// previously enabled.
///
/// If the return value is `true`, the caller must eventually call
/// [`enable_irq`] to restore the interrupt state.
#[allow(clippy::needless_return)]
pub fn ensure_irq_disabled() -> bool
{
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let rflags: u64;
        core::arch::asm!(
            "pushfq",
            "pop {}",
            out(reg) rflags,
            options(preserves_flags)
        );
        // IF (Interrupt Flag) is bit 9 of the RFLAGS register.
        let was_enabled = (rflags & (1 << 9)) != 0;
        if was_enabled
        {
            core::arch::asm!("cli", options(nomem, preserves_flags));
        }
        return was_enabled;
    }

    #[cfg(target_arch = "x86")]
    unsafe {
        let eflags: u32;
        core::arch::asm!(
            "pushfd",
            "pop {}",
            out(reg) eflags,
            options(preserves_flags)
        );
        let was_enabled = (eflags & (1 << 9)) != 0;
        if was_enabled
        {
            core::arch::asm!("cli", options(nomem, preserves_flags));
        }
        return was_enabled;
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let daif: u64;
        core::arch::asm!("mrs {}, daif", out(reg) daif, options(nomem, preserves_flags));
        // I bit (IRQ mask) is bit 7. It is 0 when enabled, 1 when disabled.
        let was_enabled = (daif & (1 << 7)) == 0;
        if was_enabled
        {
            // daifset #2 sets the I bit (disables IRQs)
            core::arch::asm!(
                "msr daifset, #2",
                options(nomem, preserves_flags)
            );
        }
        return was_enabled;
    }

    #[cfg(target_arch = "arm")]
    unsafe {
        let cpsr: u32;
        core::arch::asm!("mrs {}, cpsr", out(reg) cpsr, options(nomem, preserves_flags));
        // I bit is bit 7. 0 = enabled, 1 = disabled.
        let was_enabled = (cpsr & (1 << 7)) == 0;
        if was_enabled
        {
            core::arch::asm!("cpsid i", options(nomem, preserves_flags));
        }
        return was_enabled;
    }

    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        let sstatus: usize;
        core::arch::asm!("csrr {}, sstatus", out(reg) sstatus, options(nomem, preserves_flags));
        // SIE (Supervisor Interrupt Enable) is bit 1.
        let was_enabled = (sstatus & (1 << 1)) != 0;
        if was_enabled
        {
            // Clear SIE bit (disables IRQs)
            core::arch::asm!(
                "csrc sstatus, 2",
                options(nomem, preserves_flags)
            );
        }
        return was_enabled;
    }

    #[cfg(target_arch = "mips")]
    unsafe {
        let status: u32;
        core::arch::asm!("mfc0 {}, $12", out(reg) status, options(nomem, preserves_flags));
        // IE (Interrupt Enable) is bit 0 of the Status register.
        let was_enabled = (status & 1) != 0;
        if was_enabled
        {
            let new_status = status & !1;
            core::arch::asm!("mtc0 {}, $12", in(reg) new_status, options(nomem, preserves_flags));
        }
        return was_enabled;
    }

    #[cfg(target_arch = "powerpc64")]
    unsafe {
        let msr: usize;
        core::arch::asm!("mfmsr {}", out(reg) msr, options(nomem, preserves_flags));
        // EE (External Interrupt Enable) is IBM bit 16.
        // In 64-bit standard notation (LSB = 0), this is bit 47 (63 - 16).
        let was_enabled = (msr & (1 << 47)) != 0;
        if was_enabled
        {
            core::arch::asm!("wrteei 0", options(nomem, preserves_flags));
        }
        return was_enabled;
    }

    #[cfg(target_arch = "powerpc")]
    unsafe {
        let msr: usize;
        core::arch::asm!("mfmsr {}", out(reg) msr, options(nomem, preserves_flags));
        // EE is IBM bit 16. In 32-bit notation (LSB = 0), this is bit 15 (31 -
        // 16).
        let was_enabled = (msr & (1 << 15)) != 0;
        if was_enabled
        {
            core::arch::asm!("wrteei 0", options(nomem, preserves_flags));
        }
        return was_enabled;
    }

    #[cfg(target_arch = "wasm32")]
    {
        // WASM doesn't have traditional hardware interrupts.
        return false;
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv32",
        target_arch = "riscv64",
        target_arch = "mips",
        target_arch = "powerpc",
        target_arch = "powerpc64",
        target_arch = "wasm32",
    )))]
    {
        compile_error!("Unsupported architecture for IRQ manipulation");
    }
}

/// Unconditionally enables IRQs.
///
/// # Safety
///
/// This should only be called if interrupts were previously enabled and
/// disabled via [`ensure_irq_disabled`]. Calling this when interrupts were
/// originally disabled may break system invariants.
pub fn enable_irq()
{
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("sti", options(nomem, preserves_flags));
    }

    #[cfg(target_arch = "x86")]
    unsafe {
        core::arch::asm!("sti", options(nomem, preserves_flags));
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        // daifclr #2 clears the I bit (enables IRQs)
        core::arch::asm!("msr daifclr, #2", options(nomem, preserves_flags));
    }

    #[cfg(target_arch = "arm")]
    unsafe {
        core::arch::asm!("cpsie i", options(nomem, preserves_flags));
    }

    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        // Set SIE bit (enables IRQs)
        core::arch::asm!("csrs sstatus, 2", options(nomem, preserves_flags));
    }

    #[cfg(target_arch = "mips")]
    unsafe {
        let status: u32;
        core::arch::asm!("mfc0 {}, $12", out(reg) status, options(nomem, preserves_flags));
        let new_status = status | 1;
        core::arch::asm!("mtc0 {}, $12", in(reg) new_status, options(nomem, preserves_flags));
    }

    #[cfg(any(target_arch = "powerpc", target_arch = "powerpc64"))]
    unsafe {
        core::arch::asm!("wrteei 1", options(nomem, preserves_flags));
    }

    #[cfg(target_arch = "wasm32")]
    {
        // No-op
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "riscv32",
        target_arch = "riscv64",
        target_arch = "mips",
        target_arch = "powerpc",
        target_arch = "powerpc64",
        target_arch = "wasm32",
    )))]
    {
        compile_error!("Unsupported architecture for IRQ manipulation");
    }
}

/// A lock policy that simply disables interrupts on acquisition and restores
/// them on release.
///
/// This is useful for protecting per-CPU data or short critical sections where
/// the only source of contention is the local interrupt handler.
#[derive(Debug, Default)]
pub struct Irq;

unsafe impl LockPolicy for Irq
{
    type Error = Infallible;
    type Meta = bool;

    unsafe fn try_lock(
        &self,
        _current_iteration: usize,
    ) -> crate::LockResult<Self::Meta, Self::Error>
    {
        Ok(LockStatus::Done(ensure_irq_disabled()))
    }

    unsafe fn free(&self, meta: &Self::Meta)
    {
        if *meta
        {
            enable_irq();
        }
    }
}

/// A composite lock policy that wraps a [`SharingPolicy`] and disables
/// interrupts during exclusive (writer) acquisition.
///
/// # Design Note
///
/// This implementation **only** disables IRQs for exclusive (`try_lock`)
/// access. Shared (`try_share`) access does **not** disable IRQs. This is safe
/// *only if* your interrupt handlers never attempt to acquire an exclusive
/// (write) lock on the same resource. If an interrupt handler might need write
/// access, you must ensure that readers also disable IRQs to prevent deadlocks.
#[derive(Debug, Default)]
pub struct SharexIrq<L>(pub L)
where L: SharingPolicy;

unsafe impl<L> LockPolicy for SharexIrq<L>
where L: SharingPolicy
{
    type Error = L::Error;
    type Meta = (bool, L::Meta);

    unsafe fn try_lock(
        &self,
        current_iteration: usize,
    ) -> crate::LockResult<Self::Meta, Self::Error>
    {
        let irq_was_enabled = ensure_irq_disabled();

        match unsafe { self.0.try_lock(current_iteration) }
        {
            Ok(LockStatus::Done(x)) =>
            {
                Ok(LockStatus::Done((irq_was_enabled, x)))
            },
            Ok(LockStatus::Fail) =>
            {
                // FIX: We must restore IRQ state if acquisition fails,
                // otherwise we leave the CPU with interrupts permanently
                // disabled!
                if irq_was_enabled
                {
                    enable_irq();
                }
                Ok(LockStatus::Fail)
            },
            Err(e) =>
            {
                if irq_was_enabled
                {
                    enable_irq();
                }
                Err(e)
            },
        }
    }

    unsafe fn free(&self, meta: &Self::Meta)
    {
        unsafe {
            self.0.free(&meta.1);
        }

        if meta.0
        {
            enable_irq();
        }
    }
}

unsafe impl<L> SharingPolicy for SharexIrq<L>
where L: SharingPolicy
{
    fn try_share(
        &self,
        current_iteration: usize,
    ) -> crate::LockResult<Self::Meta, Self::Error>
    {
        match self.0.try_share(current_iteration)
        {
            // We didn't disable IRQs for readers, so we pass `false` to
            // ensure `free_share` doesn't erroneously enable them.
            Ok(LockStatus::Done(meta)) => Ok(LockStatus::Done((false, meta))),
            Ok(LockStatus::Fail) => Ok(LockStatus::Fail),
            Err(error) => Err(error),
        }
    }

    fn free_share(&self, meta: &Self::Meta)
    {
        self.0.free_share(&meta.1);
    }
}
