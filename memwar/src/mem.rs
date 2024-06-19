use std::ffi::c_void;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, Sub};
use std::ptr::{addr_of_mut, null_mut};

use winapi::shared::minwindef::DWORD;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::memoryapi::{
    ReadProcessMemory, VirtualAlloc, VirtualAllocEx, VirtualFree, VirtualFreeEx, WriteProcessMemory,
};
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::winnt::{HANDLE, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE};

/// Required wrapper struct for sharing pointers between threads.
#[derive(Copy, Clone)]
pub struct CVoidPtr(pub *mut c_void);

unsafe impl Send for CVoidPtr {}

#[derive(Debug, Clone)]
pub struct Vector2(pub f32, pub f32);

impl Vector2 {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_from(base: *mut c_void, alloc: &Allocation) -> Result<Self, u32> {
        Ok(Self(alloc.read_f32(base)?, alloc.read_f32(base.add(4))?))
    }

    pub fn len(&self) -> f32 {
        (self.0.powf(2.0) + self.1.powf(2.0)).sqrt()
    }

    pub fn as_normalized(&self) -> Self {
        let len = self.len();
        Self(self.0 / len, self.1 / len)
    }
}

#[derive(Debug, Clone)]
pub struct Vector3(pub f32, pub f32, pub f32);

impl Vector3 {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_from(base: *mut c_void, alloc: &Allocation) -> Result<Self, u32> {
        Ok(Self(
            alloc.read_f32(base)?,
            alloc.read_f32(base.add(4))?,
            alloc.read_f32(base.add(8))?,
        ))
    }

    pub fn len(&self) -> f32 {
        (self.0.powf(2.0) + self.1.powf(2.0) + self.2.powf(2.0)).sqrt()
    }

    pub fn as_normalized(&self) -> Self {
        let len = self.len();
        Self(self.0 / len, self.1 / len, self.2 / len)
    }
}

impl Sub for Vector3 {
    type Output = Vector3;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }
}

impl Add for Vector3 {
    type Output = Vector3;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

/// Required wrapper struct for sending [Allocation]s across threads.
#[derive(Clone, Copy)]
pub struct SendAlloc {
    h_process: CVoidPtr,
    p_base: CVoidPtr,
}

impl SendAlloc {
    pub const fn new(h_process: CVoidPtr, p_base: CVoidPtr) -> Self {
        Self { h_process, p_base }
    }

    pub const fn p_base(&self) -> CVoidPtr {
        self.p_base
    }
}

/// A wrapper struct for more direct approaches to the Read/WriteProcessMemory API functions.
pub struct Allocation {
    h_process: HANDLE,
    base: *mut c_void,
}

impl Allocation {
    /// Frees this remote allocation and consumes self.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn free_remote(self) -> Result<(), DWORD> {
        if VirtualFreeEx(self.h_process, self.base, 0, MEM_RELEASE) == 0 {
            return Err(GetLastError());
        }
        Ok(())
    }

    /// Frees this allocation and consumes self.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn free(self) -> Result<(), DWORD> {
        if VirtualFree(self.base, 0, MEM_RELEASE) == 0 {
            return Err(GetLastError());
        }
        Ok(())
    }

    /// Reads a [f32] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_f32(&self, addr: *mut c_void) -> Result<f32, DWORD> {
        let buf: [u8; 4] = self.read_const(addr)?;
        Ok(f32::from_le_bytes(buf))
    }

    /// Reads a [f64] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_f64(&self, addr: *mut c_void) -> Result<f64, DWORD> {
        let buf: [u8; 8] = self.read_const(addr)?;
        Ok(f64::from_le_bytes(buf))
    }

    /// Reads an [i16] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_i16(&self, addr: *mut c_void) -> Result<i16, DWORD> {
        let buf: [u8; 2] = self.read_const(addr)?;
        Ok(i16::from_le_bytes(buf))
    }

    /// Reads an [i32] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_i32(&self, addr: *mut c_void) -> Result<i32, DWORD> {
        let buf: [u8; 4] = self.read_const(addr)?;
        Ok(i32::from_le_bytes(buf))
    }

    /// Reads an [i64] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_i64(&self, addr: *mut c_void) -> Result<i64, DWORD> {
        let buf: [u8; 8] = self.read_const(addr)?;
        Ok(i64::from_le_bytes(buf))
    }

    /// Reads an [u8] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_u8(&self, addr: *mut c_void) -> Result<u8, DWORD> {
        let buf: [u8; 1] = self.read_const(addr)?;
        Ok(buf[0])
    }

    /// Reads an [u16] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_u16(&self, addr: *mut c_void) -> Result<u16, DWORD> {
        let buf: [u8; 2] = self.read_const(addr)?;
        Ok(u16::from_le_bytes(buf))
    }

    /// Reads an [u32] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_u32(&self, addr: *mut c_void) -> Result<u32, DWORD> {
        let buf: [u8; 4] = self.read_const(addr)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Reads an [u64] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_u64(&self, addr: *mut c_void) -> Result<u64, DWORD> {
        let buf: [u8; 8] = self.read_const(addr)?;
        Ok(u64::from_le_bytes(buf))
    }

    /// Reads an [u128] from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_u128(&self, addr: *mut c_void) -> Result<u128, DWORD> {
        let buf: [u8; 16] = self.read_const(addr)?;
        Ok(u128::from_le_bytes(buf))
    }

    /// Reads a constant amount of bytes into an array from the given address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_const<const N: usize>(&self, addr: *mut c_void) -> Result<[u8; N], DWORD> {
        let mut buf = [0; N];

        if self.read(addr, buf.as_mut_ptr() as _, N)? == 0 {
            return Err(GetLastError());
        }
        Ok(buf)
    }

    /// Reads `buf_size` at the given address into the provided buffer.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read(
        &self,
        addr: *mut c_void,
        buf: *mut c_void,
        buf_size: usize,
    ) -> Result<usize, DWORD> {
        let mut read = 0;

        if ReadProcessMemory(self.h_process, addr, buf, buf_size, &mut read) == 0 {
            return Err(GetLastError());
        }
        Ok(read)
    }

    /// Dereferences a multi-level pointer.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn deref_chain_with_base<const N: usize>(
        &self,
        base: *mut c_void,
        offsets: [usize; N],
    ) -> Result<*mut c_void, DWORD> {
        self.deref_chain(base.add(self.base as _), offsets)
    }

    /// Dereferences a multi-level pointer.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn deref_chain<const N: usize>(
        &self,
        base: *mut c_void,
        offsets: [usize; N],
    ) -> Result<*mut c_void, DWORD> {
        let mut addr = base;
        let mut tmp = 0;

        for (i, offset) in offsets.iter().enumerate() {
            if i == 0
                && ReadProcessMemory(
                    self.h_process,
                    addr,
                    addr_of_mut!(tmp) as _,
                    size_of::<usize>(),
                    null_mut(),
                ) == 0
            {
                return Err(GetLastError());
            }

            addr = (offset + tmp) as *mut _;

            if ReadProcessMemory(
                self.h_process,
                addr,
                addr_of_mut!(tmp) as _,
                size_of::<usize>(),
                null_mut(),
            ) == 0
            {
                return Err(GetLastError());
            }
        }
        Ok(addr)
    }

    /// Reads a [bool] at the given offset.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_bool_offset(&self, offset: usize) -> Result<bool, DWORD> {
        let mut buf = [0; 1];
        self.read_offset(offset, buf.as_mut_ptr() as _, 1)?;
        Ok(buf[0] > 0)
    }

    /// Reads an [u32] at the given offset.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_u32_offset(&self, offset: usize) -> Result<u32, DWORD> {
        let mut buf = [0; 4];
        self.read_offset(offset, buf.as_mut_ptr() as _, 4)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Reads a [f32] at the given offset.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_f32_offset(&self, offset: usize) -> Result<f32, DWORD> {
        let mut buf = [0; 4];
        self.read_offset(offset, buf.as_mut_ptr() as _, 4)?;
        Ok(f32::from_le_bytes(buf))
    }

    /// Reads the data into the given buffer.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_at_base(&self, buf: *mut c_void, buf_size: usize) -> Result<usize, DWORD> {
        self.read_offset(0, buf, buf_size)
    }

    /// Reads the data at the allocation base plus the offset into the given buffer.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_offset(
        &self,
        offset: usize,
        buf: *mut c_void,
        buf_size: usize,
    ) -> Result<usize, DWORD> {
        let mut read = 0;

        if ReadProcessMemory(
            self.h_process,
            self.base.add(offset),
            buf,
            buf_size as _,
            &mut read,
        ) == 0
        {
            return Err(GetLastError());
        }
        Ok(read)
    }

    /// Fully writes the given data to this allocation in buffers of `buf_size`, else returns an
    /// [Err] containing the last OS error.
    ///
    /// Was designed for large write operations.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_all_bytes_buffered(
        &self,
        data: &[u8],
        buf_size: usize,
    ) -> Result<(), DWORD> {
        self.write_all_bytes_buffered_offset(0, data, buf_size)
    }

    /// Fully writes the given data to this allocation, (offset by the `offset` parameter), in
    /// buffers of `buf_size`, else returns an [Err] containing the last OS error.
    ///
    /// Was designed for large write operations.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_all_bytes_buffered_offset(
        &self,
        offset: usize,
        data: &[u8],
        buf_size: usize,
    ) -> Result<(), DWORD> {
        let mut buf: Vec<u8> = Vec::with_capacity(buf_size);
        let mut remaining = data.len();
        let mut total_written = 0;
        let mut written;

        while remaining > 0 {
            let real_remains = remaining.min(buf_size);

            buf.set_len(real_remains);
            buf.copy_from_slice(&data[total_written..total_written + real_remains]);

            written = self.write_offset(total_written + offset, buf.as_ptr() as _, real_remains)?;
            total_written += written;
            remaining -= written;
        }
        Ok(())
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write(
        &self,
        addr: *mut c_void,
        data: *mut c_void,
        data_size: usize,
    ) -> Result<usize, DWORD> {
        let mut written = 0;

        if WriteProcessMemory(self.h_process, addr, data, data_size, &mut written) == 0 {
            return Err(GetLastError());
        }
        Ok(written)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_u32(&self, addr: *mut c_void, data: u32) -> Result<usize, DWORD> {
        self.write(addr, data.to_le_bytes().as_ptr() as _, 4)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_f32(&self, addr: *mut c_void, data: f32) -> Result<usize, DWORD> {
        self.write(addr, data.to_le_bytes().as_ptr() as _, 4)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_i32(&self, addr: *mut c_void, data: i32) -> Result<usize, DWORD> {
        self.write(addr, data.to_le_bytes().as_ptr() as _, 4)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_u16(&self, addr: *mut c_void, data: u16) -> Result<usize, DWORD> {
        self.write(addr, data.to_le_bytes().as_ptr() as _, 2)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_at_base(
        &self,
        data: *mut c_void,
        data_size: usize,
    ) -> Result<usize, DWORD> {
        self.write_offset(0, data, data_size)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_offset(
        &self,
        offset: usize,
        data: *mut c_void,
        data_size: usize,
    ) -> Result<usize, DWORD> {
        self.write(self.base.add(offset), data, data_size)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_u32_offset(&self, offset: usize, data: u32) -> Result<usize, DWORD> {
        self.write_offset(offset, data.to_le_bytes().as_ptr() as _, 4)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_f32_offset(&self, offset: usize, data: f32) -> Result<usize, DWORD> {
        self.write_offset(offset, data.to_le_bytes().as_ptr() as _, 4)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn write_i32_offset(&self, offset: usize, data: i32) -> Result<usize, DWORD> {
        self.write_offset(offset, data.to_le_bytes().as_ptr() as _, 4)
    }

    /// Returns a pointer to the base of this allocation.
    pub const fn inner(&self) -> *mut c_void {
        self.base
    }

    /// Allocates memory in a remote process without a specific base address. The OS will choose the
    /// address instead.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn alloc_remote_anywhere(h_process: HANDLE, size: usize) -> Result<Self, DWORD> {
        Self::alloc_remote(h_process, null_mut(), size)
    }

    /// Allocates memory in a remote process at the specified base address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn alloc_remote(
        h_process: HANDLE,
        base_addr: *mut c_void,
        size: usize,
    ) -> Result<Self, DWORD> {
        let base = VirtualAllocEx(
            h_process,
            base_addr,
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        if base.is_null() {
            return Err(GetLastError());
        }
        Ok(Self::existing(h_process, base))
    }

    /// Allocates memory in the current process at the specified base address.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn alloc(base_addr: *mut c_void, size: usize) -> Result<Self, DWORD> {
        let h_process = GetCurrentProcess();

        if h_process == INVALID_HANDLE_VALUE {
            return Err(GetLastError());
        }

        let base = VirtualAlloc(
            base_addr,
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        if base.is_null() {
            return Err(GetLastError());
        }
        Ok(Self::existing(h_process, base))
    }

    pub const fn existing(h_process: HANDLE, base: *mut c_void) -> Self {
        Self { h_process, base }
    }
}

impl Debug for Allocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}", self.base as usize)
    }
}

impl From<SendAlloc> for Allocation {
    fn from(value: SendAlloc) -> Self {
        Self {
            h_process: value.h_process.0,
            base: value.p_base().0,
        }
    }
}
