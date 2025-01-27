use memmap2::Mmap;
use minidump::{Error, Minidump, MinidumpSystemInfo, MinidumpThreadList};
use std::fs::File;
use std::path::Path;

pub struct MinidumpReader<'a> {
    dump: Minidump<'a, &'a [u8]>, // Minidump borrows from the mmap
}

impl<'a> MinidumpReader<'a> {
    /// Create a new MinidumpReader from a file path
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self, Error> {
        // Open the file and create a memory map
        let file = File::open(file_path).unwrap();
        let mmap = unsafe { Mmap::map(&file).unwrap() };

        // Convert the mmap into a 'static reference
        let mmap_ref: &'a [u8] = unsafe { std::mem::transmute(&*mmap) };

        // Create a Minidump from the memory map
        let dump = Minidump::read(mmap_ref)?;

        Ok(Self { dump })
    }

    /// Read a specified amount of bytes from a given virtual memory address
    pub fn read_virtual_memory(&self, address: u64, size: usize) -> Result<Vec<u8>, Error> {
        let memory_list = self.dump.get_memory().unwrap();

        println!("Got stream");
        if let Some(memory) = memory_list.memory_at_address(address) {
            let offset = address - memory.base_address();
            let end = (offset + size as u64) as usize;
            if memory.memory_range().unwrap().contains(address) {
                return Ok(memory.bytes()[offset as usize..end].to_vec());
            }

            eprintln!("Specified range exceeds memory region bounds");
        } else {
            eprintln!("Memory region not found for the specified address");
        }

        Err(Error::StreamReadFailure)
    }

    /// Get a list of threads
    pub fn get_threads(&self) -> Result<MinidumpThreadList<'_>, Error> {
        self.dump.get_stream::<MinidumpThreadList>()
    }

    /// Get system information
    pub fn get_system_info(&self) -> Result<MinidumpSystemInfo, Error> {
        self.dump.get_stream::<MinidumpSystemInfo>()
    }
}

fn main() {
    // Replace this with the path to your minidump file
    let minidump_path = "";
    let virtual_address = 0x7ff95f9b1000; // Example virtual address
    let byte_count = 200; // Number of bytes to read

    match MinidumpReader::new(minidump_path) {
        Ok(reader) => {
            match reader.read_virtual_memory(virtual_address, byte_count) {
                Ok(data) => {
                    println!(
                        "Read {} bytes from virtual address {:x}:",
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
                    println!("OS: {} {}", system_info.os, system_info.os);
                }
                Err(e) => eprintln!("Error accessing system info: {}", e),
            }
        }
        Err(e) => eprintln!("Failed to load minidump: {}", e),
    }
}
