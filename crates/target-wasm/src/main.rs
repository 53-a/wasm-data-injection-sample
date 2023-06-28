use std::{ptr::slice_from_raw_parts_mut, alloc::Layout};

use anyhow::{Result, anyhow};

#[cfg(target_arch = "wasm32")]

extern "C" {
    #[link_name = "__core_memory_init_dummy"]
    fn memory_init_dummy(dest: *mut u8, offset: u32, size: u32);
}

#[cfg(target_arch = "wasm32")]
#[export_name = "__core_memory_init"] // Make this function exported to save the name as export name
#[inline(never)]
fn memory_init(dest: *mut u8, offset: u32, size: u32) {
    // Call a dummy imported function to prevent the compiler from optimizing this function
    unsafe { memory_init_dummy(dest, offset, size) };
}

#[cfg(not(target_arch = "wasm32"))]
fn memory_init(_dest: *mut u8, _offset: u32, _size: u32) {}

fn load_from_data(offset: u32, layout: Layout) -> Result<*mut u8> {
    unsafe {
        let buf = std::alloc::alloc(layout);
        memory_init(buf, offset, layout.size() as u32);
        Ok(buf)
    }
}

fn load_slice_from_data<T>(offset: u32, size: u32) -> Result<*mut [T]> {
    let layout = Layout::array::<T>(size as usize)?;
    let ptr = load_from_data(offset, layout)?;
    Ok(slice_from_raw_parts_mut(ptr as *mut T, size as usize))
    
}

fn main() -> Result<()> {
    let size_slice = unsafe {
        load_slice_from_data(0, std::mem::size_of::<u32>() as u32)?.as_ref()
    }.ok_or(anyhow!("could not load data"))?;

    let size = u32::from_be_bytes(size_slice.try_into()?);

    let str_ptr = load_slice_from_data::<u8>(std::mem::size_of::<u32>() as u32, size)?;
    let cstr = unsafe { std::ffi::CString::from_raw(str_ptr as *mut std::os::raw::c_char)};
    let str = cstr.to_str()?;
    println!("{str}");
    Ok(())
}
