use alloc::boxed::Box;
use alloc::vec::Vec;

use bootproto::{
    BootInfo, MemoryRange, MemoryRegion, MemoryRegionKind, PointerRange, BOOTINFO_VERSION,
};
use log::{error, info};
use sha2::{Digest, Sha256};
use uefi::prelude::*;
use uefi::proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, RegularFile};
use uefi::table::boot::{AllocateType, MemoryMapIter, MemoryType};
use uefi::{cstr16, CStr16};

const KERNEL_PATH: &CStr16 = cstr16!(r"\EFI\RUSTCORE\kernel.elf");
const BOOTFS_PATH: &CStr16 = cstr16!(r"\EFI\RUSTCORE\bootfs.bin");
const PAGE_SIZE: usize = 4096;
const STACK_PAGES: usize = 16; // 64 KiB kernel bootstrap stack

#[global_allocator]
static ALLOCATOR: uefi::alloc::System = uefi::alloc::System;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("UEFI loader panic: {info}");
    loop {
        core::hint::spin_loop();
    }
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    error!("allocation failure: {layout:?}");
    loop {
        core::hint::spin_loop();
    }
}

#[entry]
pub fn efi_main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    if let Err(err) = efi_main_inner(image_handle, &mut system_table) {
        error!("loader failure: {:?}", err);
        Status::ABORTED
    } else {
        Status::SUCCESS
    }
}

#[derive(Debug)]
enum LoaderError {
    Services(Status),
    Filesystem(Status),
    File(Status),
    InvalidKernel(&'static str),
    Memory(Status),
    ExitBootServices(Status),
}

impl From<uefi::Status> for LoaderError {
    fn from(status: uefi::Status) -> Self {
        LoaderError::Services(status)
    }
}

fn efi_main_inner(
    image_handle: Handle,
    system_table: &mut SystemTable<Boot>,
) -> Result<(), LoaderError> {
    uefi_services::init(system_table).map_err(LoaderError::Services)?;

    info!("rustcore: UEFI loader starting");

    let bt = system_table.boot_services();

    let mut fs = bt
        .get_image_file_system(image_handle)
        .map_err(LoaderError::Filesystem)?;
    let mut root = fs.open_volume().map_err(LoaderError::Filesystem)?;

    let kernel_image = read_file(&mut root, KERNEL_PATH).map_err(LoaderError::File)?;
    let bootfs_image = read_file(&mut root, BOOTFS_PATH).map_err(LoaderError::File)?;

    let kernel = load_kernel(bt, &kernel_image)?;
    let bootfs = stage_bootfs(bt, &bootfs_image)?;
    let stack_top = allocate_stack(bt)?;

    let digest_raw = Sha256::digest(&kernel_image);
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&digest_raw);

    let mut mem_map_buf = allocate_memory_map_buffer(bt);
    let (memory_map, map_key) = bt
        .memory_map(&mut mem_map_buf)
        .map_err(LoaderError::Services)?;

    let regions = build_memory_regions(memory_map);
    let leaked_regions: &'static [MemoryRegion] = Box::leak(regions.into_boxed_slice());

    let mut boot_info = Box::new(BootInfo {
        version: BOOTINFO_VERSION,
        flags: 0,
        stack_top,
        memory_map: PointerRange {
            base: leaked_regions.as_ptr() as u64,
            len: leaked_regions.len() as u64,
            marker: core::marker::PhantomData,
        },
        rsdp: locate_rsdp(system_table),
        bootfs: bootfs,
        kernel_digest: digest,
    });

    info!("rustcore: loaded kernel at {:#x}", kernel.entry as usize);
    info!("rustcore: bootfs staged at {:#x}", boot_info.bootfs.base);

    system_table
        .exit_boot_services(image_handle, map_key)
        .map_err(LoaderError::ExitBootServices)?;

    let boot_info_ptr: *const BootInfo = Box::into_raw(boot_info);

    unsafe {
        (kernel.entry)(boot_info_ptr);
    }
}

struct LoadedKernel {
    entry: unsafe extern "C" fn(*const BootInfo) -> !,
}

fn load_kernel(bt: &BootServices, image: &[u8]) -> Result<LoadedKernel, LoaderError> {
    let elf = ElfFile::parse(image).map_err(|msg| LoaderError::InvalidKernel(msg))?;
    load_segments(bt, image, &elf)?;

    let entry_addr = elf
        .lookup_symbol_address(image, b"rustcore_entry64")
        .ok_or(LoaderError::InvalidKernel("rustcore_entry64 not found"))?;

    let entry: unsafe extern "C" fn(*const BootInfo) -> ! =
        unsafe { core::mem::transmute(entry_addr as usize) };

    Ok(LoadedKernel { entry })
}

fn stage_bootfs(bt: &BootServices, bootfs: &[u8]) -> Result<MemoryRange, LoaderError> {
    if bootfs.is_empty() {
        return Ok(MemoryRange::empty());
    }

    let pages = pages_for_len(bootfs.len());
    let phys = bt
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, pages)
        .map_err(LoaderError::Memory)?;

    unsafe {
        core::ptr::copy_nonoverlapping(bootfs.as_ptr(), phys as *mut u8, bootfs.len());
    }

    Ok(MemoryRange {
        base: phys,
        length: bootfs.len() as u64,
    })
}

fn allocate_stack(bt: &BootServices) -> Result<u64, LoaderError> {
    let phys = bt
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, STACK_PAGES)
        .map_err(LoaderError::Memory)?;
    Ok(phys + (STACK_PAGES * PAGE_SIZE) as u64)
}

fn allocate_memory_map_buffer(bt: &BootServices) -> Vec<u8> {
    let map_size = bt.memory_map_size();
    let buf_len = map_size.map_size + (map_size.entry_size * 8);
    vec![0u8; buf_len]
}

fn build_memory_regions<'a>(memory_map: MemoryMapIter<'a>) -> Vec<MemoryRegion> {
    memory_map
        .map(|desc| MemoryRegion {
            base: desc.phys_start,
            length: desc.page_count * PAGE_SIZE as u64,
            kind: classify_memory(desc.ty),
        })
        .collect()
}

fn classify_memory(ty: MemoryType) -> MemoryRegionKind {
    match ty {
        MemoryType::CONVENTIONAL => MemoryRegionKind::UsableRam,
        MemoryType::BOOT_SERVICES_CODE
        | MemoryType::BOOT_SERVICES_DATA
        | MemoryType::RUNTIME_SERVICES_CODE
        | MemoryType::RUNTIME_SERVICES_DATA
        | MemoryType::ACPI_RECLAIM
        | MemoryType::ACPI_MEMORY_NVS
        | MemoryType::MMIO
        | MemoryType::MMIO_PORT_SPACE
        | MemoryType::PAL_CODE
        | MemoryType::PERSISTENT_MEMORY
        | MemoryType::UNUSABLE => MemoryRegionKind::Reserved,
        MemoryType::LOADER_CODE | MemoryType::LOADER_DATA => MemoryRegionKind::UsableRam,
        _ => MemoryRegionKind::Reserved,
    }
}

fn locate_rsdp(system_table: &SystemTable<Boot>) -> u64 {
    system_table
        .config_table()
        .iter()
        .find(|entry| {
            entry.guid == uefi::table::cfg::ACPI2_GUID || entry.guid == uefi::table::cfg::ACPI_GUID
        })
        .map(|entry| entry.address as u64)
        .unwrap_or(0)
}

fn pages_for_len(len: usize) -> usize {
    (len + PAGE_SIZE - 1) / PAGE_SIZE
}

fn read_file(root: &mut Directory, path: &CStr16) -> Result<Vec<u8>, Status> {
    let mut file = match root.open(path, FileMode::Read, FileAttribute::empty())? {
        File::Regular(file) => file,
        _ => return Err(Status::LOAD_ERROR),
    };

    let info = file.get_boxed_info::<FileInfo>()?;
    let size = info.file_size() as usize;
    let mut buffer = vec![0u8; size];
    file.read(&mut buffer)?;
    Ok(buffer)
}

fn load_segments(bt: &BootServices, image: &[u8], elf: &ElfFile) -> Result<(), LoaderError> {
    for ph in elf.program_headers(image) {
        if ph.ph_type != PT_LOAD {
            continue;
        }

        let dest = ph.phys_addr as usize;
        let file_size = ph.file_size as usize;
        let mem_size = ph.mem_size as usize;
        let pages = pages_for_len(mem_size);

        if pages == 0 {
            continue;
        }

        bt.allocate_pages(
            AllocateType::Address(ph.phys_addr),
            MemoryType::LOADER_DATA,
            pages,
        )
        .map_err(LoaderError::Memory)?;

        let src = &image[ph.offset as usize..ph.offset as usize + file_size];
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), dest as *mut u8, file_size);
            if mem_size > file_size {
                core::ptr::write_bytes((dest + file_size) as *mut u8, 0, mem_size - file_size);
            }
        }
    }
    Ok(())
}

/// Minimal ELF64 parser tailored for the Rustcore kernel image.
struct ElfFile;

impl ElfFile {
    fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < core::mem::size_of::<ElfHeader>() {
            return Err("file too small for ELF header");
        }
        let header = unsafe { &*(bytes.as_ptr() as *const ElfHeader) };
        if &header.magic != b"\x7FELF" {
            return Err("invalid ELF magic");
        }
        if header.class != 2 || header.data != 1 {
            return Err("unsupported ELF class or endianness");
        }
        if header.machine != 0x3E {
            return Err("unsupported machine");
        }
        if header.program_entry_size != core::mem::size_of::<ProgramHeader>() as u16 {
            return Err("unexpected program header size");
        }
        if header.section_entry_size != core::mem::size_of::<SectionHeader>() as u16 {
            return Err("unexpected section header size");
        }
        Ok(ElfFile)
    }

    fn program_headers<'a>(&self, bytes: &'a [u8]) -> ProgramHeaderIter<'a> {
        ProgramHeaderIter {
            bytes,
            idx: 0,
            header: unsafe { &*(bytes.as_ptr() as *const ElfHeader) },
        }
    }

    fn section_headers<'a>(&self, bytes: &'a [u8]) -> SectionHeaderIter<'a> {
        SectionHeaderIter {
            bytes,
            idx: 0,
            header: unsafe { &*(bytes.as_ptr() as *const ElfHeader) },
        }
    }

    fn lookup_symbol_address(&self, bytes: &[u8], name: &[u8]) -> Option<u64> {
        for sh in self.section_headers(bytes) {
            if sh.sh_type != SHT_SYMTAB {
                continue;
            }
            let strtab = self.section_headers(bytes).nth(sh.sh_link as usize)?;
            let strtab_bytes = &bytes[strtab.offset as usize..][..strtab.size as usize];
            let entry_count = (sh.size / sh.ent_size) as usize;
            for idx in 0..entry_count {
                let sym_offset = sh.offset as usize + idx * sh.ent_size as usize;
                let sym = unsafe { &*(bytes.as_ptr().add(sym_offset) as *const Symbol) };
                let name_off = sym.name as usize;
                if name_off >= strtab_bytes.len() {
                    continue;
                }
                let terminator = strtab_bytes[name_off..].iter().position(|&b| b == 0)?;
                if &strtab_bytes[name_off..name_off + terminator] == name {
                    return Some(sym.value);
                }
            }
        }
        None
    }
}

#[repr(C)]
struct ElfHeader {
    magic: [u8; 4],
    class: u8,
    data: u8,
    version: u8,
    abi: u8,
    abi_version: u8,
    pad: [u8; 7],
    ty: u16,
    machine: u16,
    version2: u32,
    entry: u64,
    program_offset: u64,
    section_offset: u64,
    flags: u32,
    header_size: u16,
    program_entry_size: u16,
    program_count: u16,
    section_entry_size: u16,
    section_count: u16,
    sh_str_index: u16,
}

#[repr(C)]
struct ProgramHeader {
    ph_type: u32,
    flags: u32,
    offset: u64,
    virt_addr: u64,
    phys_addr: u64,
    file_size: u64,
    mem_size: u64,
    align: u64,
}

#[repr(C)]
struct SectionHeader {
    name: u32,
    sh_type: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    ent_size: u64,
}

#[repr(C)]
struct Symbol {
    name: u32,
    info: u8,
    other: u8,
    shndx: u16,
    value: u64,
    size: u64,
}

const PT_LOAD: u32 = 1;
const SHT_SYMTAB: u32 = 2;

struct ProgramHeaderIter<'a> {
    bytes: &'a [u8],
    idx: usize,
    header: &'a ElfHeader,
}

impl<'a> Iterator for ProgramHeaderIter<'a> {
    type Item = ProgramHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.header.program_count as usize {
            return None;
        }
        let offset = self.header.program_offset as usize
            + self.idx * self.header.program_entry_size as usize;
        self.idx += 1;
        Some(unsafe { *(self.bytes.as_ptr().add(offset) as *const ProgramHeader) })
    }
}

struct SectionHeaderIter<'a> {
    bytes: &'a [u8],
    idx: usize,
    header: &'a ElfHeader,
}

impl<'a> Iterator for SectionHeaderIter<'a> {
    type Item = SectionHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.header.section_count as usize {
            return None;
        }
        let offset = self.header.section_offset as usize
            + self.idx * self.header.section_entry_size as usize;
        self.idx += 1;
        Some(unsafe { *(self.bytes.as_ptr().add(offset) as *const SectionHeader) })
    }
}
