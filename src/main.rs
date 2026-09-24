use std::process::ExitCode;

use marlow::{GuestMemory, Kvm};

/// 2 MiB of guest RAM based at guest physical address 0 that's demand-paged.
const GUEST_RAM_BASE: u64 = 0;
const GUEST_RAM_SIZE: usize = 2 * 1024 * 1024;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("marlow: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> marlow::Result<()> {
    let kvm = Kvm::new()?;
    println!("opened /dev/kvm, API version OK");

    let mut vm = kvm.create_vm()?;
    println!("created VM");

    let mem = GuestMemory::new(GUEST_RAM_BASE, GUEST_RAM_SIZE)?;
    println!(
        "mapped {} bytes of guest RAM at host {:#x}",
        mem.size(),
        mem.host_addr()
    );

    vm.set_user_memory_region(0, mem)?;
    let mem = vm.memory(0).expect("slot 0 was just registered");
    println!(
        "registered memslot 0: GPA {:#x}..{:#x}",
        mem.guest_physical_addr(),
        mem.guest_physical_addr() + mem.size() as u64
    );

    let pattern = [0xDE, 0xAD, 0xBE, 0xEF];
    mem.write(0x1000, &pattern)?;
    let mut got = [0u8; 4];
    mem.read(0x1000, &mut got)?;
    assert_eq!(got, pattern);
    println!("host read/write roundtrip at GPA 0x1000 OK: {got:02x?}");

    Ok(())
}
