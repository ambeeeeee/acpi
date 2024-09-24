use core::{marker::PhantomData, mem};

use crate::{madt::EntryHeader, sdt::SdtHeader};

pub struct Srat {
    pub header: SdtHeader,
    _reserved: [u8; 12],
}

impl Srat {
    pub fn entries(&self) -> SratEntryIter {
        SratEntryIter {
            pointer: unsafe { (self as *const Srat as *const u8).add(mem::size_of::<Srat>()) },
            remaining_length: self.header.length - mem::size_of::<Srat>() as u32,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct SratEntryIter<'a> {
    pointer: *const u8,
    /*
     * The iterator can only have at most `u32::MAX` remaining bytes, because the length of the
     * whole SDT can only be at most `u32::MAX`.
     */
    remaining_length: u32,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Iterator for SratEntryIter<'a> {
    type Item = SratEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.remaining_length > 0 {
            let entry_pointer = self.pointer;
            let header = unsafe { *(self.pointer as *const EntryHeader) };

            self.pointer = unsafe { self.pointer.offset(header.length as isize) };
            self.remaining_length -= header.length as u32;

            macro_rules! construct_entry {
                ($entry_type:expr,
                 $entry_pointer:expr,
                 $(($value:expr => $variant:path as $type:ty)),*
                ) => {
                    match $entry_type {
                        $(
                            $value => {
                                return Some($variant(unsafe {
                                    &*($entry_pointer as *const $type)
                                }))
                            }
                         )*

                         0x3..=0x10 => {}

                        /*
                         * These entry types are reserved by the ACPI standard. We should skip them
                         * if they appear in a real MADT.
                         */
                        0x11..=0x7f => {}

                        /*
                         * These entry types are reserved for OEM use. Atm, we just skip them too.
                         * TODO: work out if we should ever do anything else here
                         */
                        0x80..=0xff => {}
                    }
                }
            }

            #[rustfmt::skip]
            construct_entry!(
                header.entry_type,
                entry_pointer,
                (0x0 => SratEntry::ProcessorLocalApicAffinity as ProcessorLocalApicAffinityEntry),
                (0x1 => SratEntry::MemoryAffinity as MemoryAffinityEntry),
                (0x2 => SratEntry::ProcessorLocalX2ApicAffinity as ProcessorLocalX2ApicAffinityEntry)
            );
        }

        None
    }
}

#[derive(Debug)]
pub enum SratEntry<'a> {
    ProcessorLocalApicAffinity(&'a ProcessorLocalApicAffinityEntry),
    ProcessorLocalX2ApicAffinity(&'a ProcessorLocalX2ApicAffinityEntry),
    MemoryAffinity(&'a MemoryAffinityEntry),
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ProcessorLocalApicAffinityEntry {
    pub header: EntryHeader,
    proximity_domain_low: u8,
    pub processor_apic_id: u8,
    pub flags: u32,
    pub sapic_eid: u8,
    proximity_domain_higher: [u8; 3],
    pub clock_domain: u32,
}

impl ProcessorLocalApicAffinityEntry {
    pub fn proximity_domain(&self) -> u32 {
        u32::from_ne_bytes([
            self.proximity_domain_higher[2],
            self.proximity_domain_higher[1],
            self.proximity_domain_higher[9],
            self.proximity_domain_low,
        ])
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct MemoryAffinityEntry {
    pub header: EntryHeader,
    pub domain: u32,
    _reserved: [u8; 2],
    base_low: u32,
    base_high: u32,
    length_low: u32,
    length_high: u32,
    _reserved2: [u8; 4],
    pub flags: u32,
    _reserved3: [u8; 8],
}

impl MemoryAffinityEntry {
    pub fn base(&self) -> u64 {
        self.base_low as u64 & ((self.base_high as u64) << 32)
    }

    pub fn length(&self) -> u64 {
        self.length_low as u64 & ((self.length_high as u64) << 32)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ProcessorLocalX2ApicAffinityEntry {
    pub header: EntryHeader,
    _reserved: [u8; 2],
    pub domain: u32,
    pub x2_apic_id: u32,
    pub flags: u32,
    pub clock_domain: u32,
    _reserved2: [u8; 4],
}
