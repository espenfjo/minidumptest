use memmap2::Mmap;
use minidump::{Error, Minidump, MinidumpMemoryList, MinidumpSystemInfo, MinidumpThreadList};
use std::fs::File;
use std::path::Path;
use std::slice;

/// A reader that holds onto the memory map and a `Minidump` that references it.
pub struct MinidumpReader {
    /// The memory map is owned here so the bytes remain valid
    /// for the lifetime of the struct.
    _mmap: Mmap,
    /// The parsed minidump, which references the mmap's bytes.
    dump: Minidump<'static, &'static [u8]>,
}

impl MinidumpReader {
    /// Create a new `MinidumpReader` from a file path.
    ///
    /// Storing the `mmap` and the parsed `dump` in the same struct
    /// ensures the bytes live at least as long as the `dump`.
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self, Error> {
        let file = File::open(file_path).map_err(|_| Error::FileNotFound)?;
        let mmap = unsafe { Mmap::map(&file).map_err(|_| Error::IoError)? };

        // We create a local slice referencing the mmap's bytes
        // (which is safe as long as `mmap` is alive).
        let dump_slice = unsafe { slice::from_raw_parts(mmap.as_ptr(), mmap.len()) };

        // Parse the minidump from that slice.
        let dump = Minidump::read(dump_slice)?;

        Ok(Self { _mmap: mmap, dump })
    }

    /// Read a specified amount of bytes from a given virtual memory address.
    pub fn read_virtual_memory(&self, address: u64, size: usize) -> Result<Vec<u8>, Error> {
        let memory_list = self.dump.get_memory().unwrap();

        let memory = memory_list
            .memory_at_address(address)
            .ok_or(Error::StreamReadFailure)?;

        // Calculate offset into this memory region
        let offset = address
            .checked_sub(memory.base_address())
            .ok_or(Error::StreamReadFailure)?;

        let bytes = memory.bytes();

        // Ensure the requested slice is entirely within the memory region
        if offset as usize + size > bytes.len() {
            return Err(Error::StreamReadFailure);
        }

        Ok(bytes[offset as usize..offset as usize + size].to_vec())
    }

    /// Get a list of threads from the minidump.
    pub fn get_threads(&self) -> Result<MinidumpThreadList<'_>, Error> {
        self.dump.get_stream::<MinidumpThreadList>()
    }

    /// Get system information from the minidump.
    pub fn get_system_info(&self) -> Result<MinidumpSystemInfo, Error> {
        self.dump.get_stream::<MinidumpSystemInfo>()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Replace this with the path to your minidump file
    let minidump_path = "";
    let virtual_address = 0x7ff95f9b1000; // Example virtual address
    let byte_count = 200; // Number of bytes to read

    let reader = MinidumpReader::new(minidump_path)?;

    // Example: Reading memory
    match reader.read_virtual_memory(virtual_address, byte_count) {
        Ok(data) => {
            println!(
                "Read {} bytes from virtual address 0x{:x}:",
                data.len(),
                virtual_address
            );
            for byte in data {
                print!("{:02x} ", byte);
            }
            println!();
        }
        Err(e) => eprintln!("Error reading virtual memory: {}", e),
    }

    // Example: Access threads
    match reader.get_threads() {
        Ok(threads) => {
            println!("Thread count: {}", threads.threads.len());
        }
        Err(e) => eprintln!("Error accessing threads: {}", e),
    }

    // Example: Access system info
    match reader.get_system_info() {
        Ok(system_info) => {
            println!(
                "System Info -> OS: {}, CPU: {}",
                system_info.os, system_info.cpu
            );
        }
        Err(e) => eprintln!("Error accessing system info: {}", e),
    }

    Ok(())
}
